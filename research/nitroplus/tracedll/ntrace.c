/* ntrace.dll: a JSONL trace of 装甲悪鬼村正 (Muramasa.exe) under wine.
 *
 * Injected by research/xuse/tracedll/xlaunch.exe. Hooks, by the addresses
 * in research/nitroplus/names.tsv (this exe only, sha256 1d67e4771a72...):
 * - run_statement 0x470a50 calls dispatch_builtin at 0x470b7c (every
 *   statement that is a call: a built-in or an NSS function) and, at
 *   0x470c9d, the text block's start 0x475000 (a <PRE> block reached).
 *   Both calls are redirected to a stub that logs and jumps on. There
 *   EBX = the NSS process object (one per function call too), ESI = the script
 *   context; the process's +0x20 is its file's index, the context's
 *   +0x1130 + index * 0x158 that file's record (+0x11c its text);
 *   +0xe8c / +0x168c + 4 i a text block's start and end in that text.
 * Times are "ms" since the DLL loaded: the script's waits are timeGetTime
 * waits in each process's thread (builtin_Wait), not frames.
 * Records ("t"): meta, cmd (one per statement dispatched, "op" its name),
 * page (one per page of a text block, its text with the tags left out). "at" =
 * "<file>:<line>", the file as the converter names it (nss/ left out),
 * the line counted from 1; "th" numbers the OS threads (one per NSS
 * process) in the order they first ran a statement (0: the main one).
 * Environment (wine passes it through):
 *   NTRACE_OUT    Windows path of the JSONL (default C:\ntrace.jsonl)
 *   NTRACE_PAGES  exit the game after this many pages (0: never)
 */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

#define CALL_DISPATCH 0x470b7c
#define DISPATCH 0x405250
#define CALL_BLOCK 0x470c9d
#define BLOCK 0x475000
#define FILE_REC(ctx, th) ((unsigned char *)(ctx) + 0x1130 + *(int *)((th) + 0x20) * 0x158)
#define FILE_NAME_AT 0 /* the file record's name, "nss/0_boot.nss" */

static FILE *out;
static LARGE_INTEGER t0, freq;
static long pages, max_pages;
static CRITICAL_SECTION lock;
static DWORD threads[64];
static unsigned char *frames_seen[256], *last_pos[256];
static int n_threads, n_frames;
void *nt_dispatch = (void *)DISPATCH, *nt_block = (void *)BLOCK;

static long ms(void) {
    LARGE_INTEGER t;
    QueryPerformanceCounter(&t);
    return (long)((t.QuadPart - t0.QuadPart) * 1000 / freq.QuadPart);
}

/* cp932 -> JSON-escaped UTF-8; tags: leave out <...>, [textNNNN] and line breaks */
static void put_str(const unsigned char *s, int n, int tags) {
    static char sj[65536], u[3 * 65536];
    static WCHAR w[65536];
    int k = 0;
    for (int i = 0; i < n && k < (int)sizeof sj; i++) {
        if (tags && s[i] == '<') {
            while (i < n && s[i] != '>') i++;
            continue;
        }
        if (tags && s[i] == '[' && i + 5 < n && !memcmp(s + i, "[text", 5)) {
            while (i < n && s[i] != ']') i++;
            continue;
        }
        if (tags && (s[i] == '\r' || s[i] == '\n')) continue;
        sj[k++] = s[i];
        if ((s[i] >= 0x81 && s[i] <= 0x9f) || (s[i] >= 0xe0 && s[i] <= 0xfc)) { /* lead byte: copy the trail as is */
            if (++i < n) sj[k++] = s[i];
        }
    }
    int nw = MultiByteToWideChar(932, 0, sj, k, w, 65536);
    int nu = WideCharToMultiByte(CP_UTF8, 0, w, nw, u, sizeof u, NULL, NULL);
    fputc('"', out);
    for (int i = 0; i < nu; i++) {
        unsigned char c = u[i];
        if (c == '"' || c == '\\') fprintf(out, "\\%c", c);
        else if (c < 0x20) fprintf(out, "\\u%04x", c);
        else fputc(c, out);
    }
    fputc('"', out);
}

/* the OS thread, numbered in the order they first ran a statement (each
 * NSS process runs on its own) */
static int thread_no(void) {
    DWORD id = GetCurrentThreadId();
    for (int i = 0; i < n_threads; i++)
        if (threads[i] == id) return i;
    if (n_threads == 64) return 63;
    threads[n_threads] = id;
    return n_threads++;
}

/* the process object run_statement runs (one per call of an NSS function too) */
static int frame_no(unsigned char *th) {
    for (int i = 0; i < n_frames; i++)
        if (frames_seen[i] == th) return i;
    if (n_frames == 256) return 255;
    frames_seen[n_frames] = th;
    return n_frames++;
}

/* "at": the file's name and the line of p in its text */
static void put_at(unsigned char *ctx, unsigned char *th, const unsigned char *p) {
    unsigned char *rec = FILE_REC(ctx, th);
    const unsigned char *base = *(unsigned char **)(rec + 0x11c);
    int line = 1;
    /* The loaded text turns the CR LF ending a text block's header lines
     * (<PRE ...>, [text0010]) and its code lines ({...}) into LF LF: a line
     * of the file is a CR LF or two bare LFs. */
    int crlf = 0, bare = 0;
    for (const unsigned char *q = base; base && q < p; q++)
        if (*q == '\n') *(q > base && q[-1] == '\r' ? &crlf : &bare) += 1;
    line += crlf + (bare + 1) / 2;
    const char *name = (const char *)rec + FILE_NAME_AT;
    if (!_strnicmp(name, "nss/", 4) || !_strnicmp(name, "nss\\", 4)) name += 4;
    fputs("\"at\":", out);
    char at[300];
    _snprintf(at, sizeof at, "%.255s:%d", name, line);
    at[sizeof at - 1] = 0;
    put_str((const unsigned char *)at, strlen(at), 0);
}

/* the length of the name if p starts a call `Name(` (a built-in or an NSS
 * function; dispatch_builtin is tried at every place run_statement stops) */
static int call_name(const unsigned char *p) {
    static const char *const words[] = {"if", "while", "for", "switch", "select", "case", "return", "else", NULL};
    int n = 0;
    if (!(isalpha(p[0]) || p[0] == '_' || p[0] >= 0x81)) return 0;
    while (n < 64 && (isalnum(p[n]) || p[n] == '_' || p[n] >= 0x81)) n += p[n] >= 0x81 ? 2 : 1;
    int k = n;
    while (p[k] == ' ' || p[k] == '\t') k++;
    if (p[k] != '(') return 0;
    for (int i = 0; words[i]; i++)
        if ((int)strlen(words[i]) == n && !memcmp(p, words[i], n)) return 0;
    return n;
}

/* a statement about to be dispatched: *pos its text */
void __cdecl nt_cmd(unsigned char *th, unsigned char **pos, unsigned char *ctx) {
    unsigned char *p = *pos;
    int k;
    EnterCriticalSection(&lock);
    k = frame_no(th);
    int n = call_name(p);
    if (n && p != last_pos[k]) { /* a call, not the same one run again */
        last_pos[k] = p;
        fprintf(out, "{\"t\":\"cmd\",\"pg\":%ld,\"ms\":%ld,\"th\":%d,", pages, ms(), thread_no());
        put_at(ctx, th, p);
        fputs(",\"op\":", out);
        put_str(p, n, 0);
        fputs("}\n", out);
        fflush(out);
    }
    LeaveCriticalSection(&lock);
}

static void page_record(unsigned char *ctx, unsigned char *th, const unsigned char *start, const unsigned char *text, int n) {
    fprintf(out, "{\"t\":\"page\",\"pg\":%ld,\"ms\":%ld,\"th\":%d,", ++pages, ms(), thread_no());
    put_at(ctx, th, start);
    fputs(",\"segs\":[", out);
    put_str(text, n, 1);
    fputs("]}\n", out);
}

/* text block i reached: its text runs from start to end. A blank line ends
 * a page and <k> shows the page so far (the converter's pages: convert.rs,
 * `pages`); tags, markers ([text0010]), code ({...}) and comment lines are
 * left out. All its pages
 * are logged now, at the block's line. */
void __cdecl nt_page(unsigned char *th, int i, unsigned char *ctx) {
    static unsigned char page[65536];
    unsigned char *start = *(unsigned char **)(th + 0xe8c + 4 * i), *end = *(unsigned char **)(th + 0x168c + 4 * i);
    int n = 0, code = 0;
    EnterCriticalSection(&lock);
    for (unsigned char *l = start; start && l < end && end - start < 60000;) {
        unsigned char *e = l;
        while (e < end && *e != '\n') e++;
        unsigned char *q = l;
        while (q < e && (*q == ' ' || *q == '\t' || *q == '\r')) q++;
        if (q == e && !code) { /* blank */
            if (n) page_record(ctx, th, start, page, n);
            n = 0;
        } else if (code || !(q + 1 < e && q[0] == '/' && q[1] == '/')) {
            for (unsigned char *c = l; c < e && n < (int)sizeof page - 2; c++) {
                int lead = (*c >= 0x81 && *c <= 0x9f) || (*c >= 0xe0 && *c <= 0xfc);
                if (code || *c == '{') { /* code, run where it stands (may span lines) */
                    code = *c != '}';
                    c += lead;
                    continue;
                }
                if (*c == '<') { /* a tag; <k> waits mid-page: the page so far is shown */
                    if ((c[1] == 'k' || c[1] == 'K') && c[2] == '>' && n) page_record(ctx, th, start, page, n);
                    while (c < e && *c != '>') c++;
                    continue;
                }
                if (*c == '[' && e - c > 5 && !memcmp(c, "[text", 5)) {
                    while (c < e && *c != ']') c++;
                    continue;
                }
                if (*c == '\r') continue;
                page[n++] = *c;
                if (lead && c + 1 < e) page[n++] = *++c;
            }
        }
        /* a bare LF LF is one line end (see put_at) */
        l = e + (e > l && e[-1] != '\r' && e + 1 < end && e[1] == '\n' ? 2 : 1);
    }
    if (n) page_record(ctx, th, start, page, n);
    fflush(out);
    if (max_pages && pages >= max_pages) TerminateProcess(GetCurrentProcess(), 0);
    LeaveCriticalSection(&lock);
}

/* at the call sites: EBX = process, ESI = context; EDI = &pos (dispatch);
 * [esp+4] = the block's index (block). Save all, log, go on to the callee. */
__asm__(".globl _nt_dispatch_stub\n_nt_dispatch_stub:\n"
        "pushal\n pushfl\n push %esi\n push %edi\n push %ebx\n call _nt_cmd\n add $12,%esp\n popfl\n popal\n"
        "jmp *_nt_dispatch\n"
        ".globl _nt_block_stub\n_nt_block_stub:\n"
        "pushal\n pushfl\n push %esi\n pushl 44(%esp)\n push %ebx\n call _nt_page\n add $12,%esp\n popfl\n popal\n"
        "jmp *_nt_block\n");
void nt_dispatch_stub(void), nt_block_stub(void);

static void redirect_call(unsigned at, void *to) {
    DWORD old;
    VirtualProtect((void *)at, 5, PAGE_EXECUTE_READWRITE, &old);
    *(int *)(at + 1) = (int)to - (int)(at + 5);
    VirtualProtect((void *)at, 5, old, &old);
}

static void install(void) {
    const char *p = getenv("NTRACE_OUT");
    out = fopen(p ? p : "C:\\ntrace.jsonl", "w");
    if (!out) return;
    InitializeCriticalSection(&lock);
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);
    if ((p = getenv("NTRACE_PAGES"))) max_pages = atol(p);
    fprintf(out, "{\"t\":\"meta\",\"schema\":1,\"engine\":\"Muramasa.exe (ntrace)\",\"max_pages\":%ld}\n", max_pages);
    if (*(unsigned char *)CALL_DISPATCH != 0xe8 || *(unsigned char *)CALL_BLOCK != 0xe8) {
        fputs("{\"t\":\"error\",\"what\":\"not the exe these addresses are for\"}\n", out);
        fflush(out);
        return;
    }
    redirect_call(CALL_DISPATCH, (void *)nt_dispatch_stub);
    redirect_call(CALL_BLOCK, (void *)nt_block_stub);
    fflush(out);
}

BOOL WINAPI DllMain(HINSTANCE h, DWORD why, LPVOID r) {
    (void)h, (void)r;
    if (why == DLL_PROCESS_ATTACH) install();
    return TRUE;
}
