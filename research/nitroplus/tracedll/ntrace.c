/* ntrace.dll: a JSONL trace of 装甲悪鬼村正 (Muramasa.exe) or 続・殺戮のジャンゴ
 * (Django.exe 1.01, unpacked) under wine.
 *
 * Injected by research/xuse/tracedll/xlaunch.exe; the exe is told by the
 * bytes at the call sites. Muramasa's hooks, by the addresses in
 * research/nitroplus/names.tsv (this exe only, sha256 1d67e4771a72...):
 * - run_statement 0x470a50 calls dispatch_builtin at 0x470b7c (every
 *   statement that is a call: a built-in or an NSS function) and, at
 *   0x470c9d, the text block's start 0x475000 (a <PRE> block reached).
 *   Both calls are redirected to a stub that logs and jumps on. There
 *   EBX = the NSS process object (one per function call too), ESI = the script
 *   context; the process's +0x20 is its file's index, the context's
 *   +0x1130 + index * 0x158 that file's record (+0x11c its text);
 *   +0xe8c / +0x168c + 4 i a text block's start and end in that text.
 * Django's (research/nitroplus/django-names.tsv; the unpacked 1.01 exe,
 * docs/games/django.md "The exe"): run_statement 0x453c00 calls
 * dispatch_builtin at 0x453cca (ESI = process, EBX = &pos, EDI = context;
 * the process's +0x1c its file's index, the file record context + 0x22c +
 * index * 0x14c, +0x114 its text). Text blocks are not run where they
 * stand (run_statement steps over them): builtin_LoadText lays out each
 * [textNNN] of a cut beforehand (the call at 0x40a3a0 returns the layout
 * of the text record in EBP: +0x100 / +0x104 the text's start and end)
 * and `Request(text, Enter)` shows it (request_enter 0x43e080, called at
 * 0x43d3d7 and 0x43d6d8 with ESI = the object, +0x808 its layout): the
 * pages are logged at that Enter, "at" the line of the block's <PRE>.
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
 *   TRACE_SPEED   N: the game runs N times as fast, "ms" in its time
 *                 (research/tools/trace_speed.h)
 *   NTRACE_SHOT_REQ  Windows path of a request file: at each text block the
 *                 DLL writes its first page's number into it (unless one is
 *                 still pending) and goes on; ntrace.sh --shots takes the
 *                 screen a moment later and deletes it (research/tools/picdiff.py)
 */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include "../../tools/trace_speed.h"

#define CALL_DISPATCH 0x470b7c
#define DISPATCH 0x405250
#define CALL_BLOCK 0x470c9d
#define BLOCK 0x475000
#define D_CALL_DISPATCH 0x453cca
#define D_DISPATCH 0x404320
#define D_AFTER_LAYOUT 0x40a3a5 /* add $0x1c,%esp; test %eax,%eax (5 bytes) after the layout call */
#define D_CALL_ENTER1 0x43d3d7
#define D_CALL_ENTER2 0x43d6d8
#define D_ENTER 0x43e080
/* the file records: context + base + index * size; the index at process + idx */
static int file_base = 0x1130, file_size = 0x158, file_idx = 0x20, file_text = 0x11c;
#define FILE_REC(ctx, th) ((unsigned char *)(ctx) + file_base + *(int *)((th) + file_idx) * file_size)
#define FILE_NAME_AT 0 /* the file record's name, "nss/0_boot.nss" */

static FILE *out;
static long pages, max_pages;
static char shot_req[MAX_PATH];
static CRITICAL_SECTION lock;
static DWORD threads[64];
static unsigned char *frames_seen[256], *last_pos[256];
static int n_threads, n_frames, page_th = -1;
static unsigned char *d_ctx; /* the script context (one for the game) */
void *nt_dispatch = (void *)DISPATCH, *nt_block = (void *)BLOCK;

static long ms(void) { return speed_ms(); } /* game time: trace_speed.h */

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
static void put_at_rec(unsigned char *rec, const unsigned char *p) {
    const unsigned char *base = *(unsigned char **)(rec + file_text);
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

static void put_at(unsigned char *ctx, unsigned char *th, const unsigned char *p) {
    put_at_rec(FILE_REC(ctx, th), p);
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
    d_ctx = ctx;
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

static void page_record(unsigned char *rec, const unsigned char *at, const unsigned char *text, int n) {
    fprintf(out, "{\"t\":\"page\",\"pg\":%ld,\"ms\":%ld,\"th\":%d,", ++pages, ms(), page_th >= 0 ? page_th : thread_no());
    put_at_rec(rec, at);
    fputs(",\"segs\":[", out);
    put_str(text, n, 1);
    fputs("]}\n", out);
}

/* text block i reached: its text runs from start to end. A blank line ends
 * a page and <k> shows the page so far (the converter's pages: convert.rs,
 * `pages`); tags, markers ([text0010]), code ({...}) and comment lines are
 * left out. All its pages
 * are logged now, at the block's line. */
static void log_pages(unsigned char *rec, const unsigned char *at, unsigned char *start, unsigned char *end) {
    static unsigned char page[65536];
    int n = 0, code = 0;
    long first = pages + 1;
    for (unsigned char *l = start; start && l < end && end - start < 60000;) {
        unsigned char *e = l;
        while (e < end && *e != '\n') e++;
        unsigned char *q = l;
        while (q < e && (*q == ' ' || *q == '\t' || *q == '\r')) q++;
        if (q == e && !code) { /* blank */
            if (n) page_record(rec, at, page, n);
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
                    if ((c[1] == 'k' || c[1] == 'K') && c[2] == '>' && n) page_record(rec, at, page, n);
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
    if (n) page_record(rec, at, page, n);
    fflush(out);
    if (*shot_req && pages >= first && GetFileAttributesA(shot_req) == INVALID_FILE_ATTRIBUTES) {
        FILE *f = fopen(shot_req, "w");
        if (f) fprintf(f, "%ld\n", first), fclose(f);
    }
    if (max_pages && pages >= max_pages) TerminateProcess(GetCurrentProcess(), 0);
}

void __cdecl nt_page(unsigned char *th, int i, unsigned char *ctx) {
    unsigned char *start = *(unsigned char **)(th + 0xe8c + 4 * i), *end = *(unsigned char **)(th + 0x168c + 4 * i);
    EnterCriticalSection(&lock);
    log_pages(FILE_REC(ctx, th), start, start, end);
    LeaveCriticalSection(&lock);
}

/* Django: the layouts builtin_LoadText made, with their text records (the
 * newest last; a freed layout's address comes back with its new record) */
static struct { void *layout; unsigned char *start, *end; } layouts[8192];
static int n_layouts;

void __cdecl nt_layout(unsigned char *rec, void *layout) {
    EnterCriticalSection(&lock);
    if (layout) {
        int i = n_layouts < 8192 ? n_layouts++ : (memmove(layouts, layouts + 1, sizeof layouts - sizeof *layouts), 8191);
        layouts[i].layout = layout;
        layouts[i].start = *(unsigned char **)(rec + 0x100);
        layouts[i].end = *(unsigned char **)(rec + 0x104) + 1;
    }
    LeaveCriticalSection(&lock);
}

/* Django: the loaded file whose text holds p */
static unsigned char *file_of(const unsigned char *p) {
    for (int i = 0; d_ctx && i < 64; i++) {
        unsigned char *rec = d_ctx + file_base + i * file_size, *base = *(unsigned char **)(rec + file_text);
        if (base && p >= base && p < base + strlen((char *)base)) return rec;
    }
    return NULL;
}

/* Django: Request(obj, Enter); a text laid out by LoadText logs its pages */
void __cdecl nt_enter(unsigned char *obj) {
    EnterCriticalSection(&lock);
    void *layout = obj ? *(void **)(obj + 0x808) : NULL;
    for (int i = n_layouts - 1; layout && i >= 0; i--) {
        if (layouts[i].layout != layout) continue;
        unsigned char *rec = file_of(layouts[i].start), *at = layouts[i].start, *base;
        if (!rec) break;
        base = *(unsigned char **)(rec + file_text);
        while (at > base && _strnicmp((char *)at, "<PRE", 4)) at--; /* the block's <PRE> */
        log_pages(rec, at, layouts[i].start, layouts[i].end);
        break;
    }
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

/* Django: dispatch (ESI = process, EBX = &pos, EDI = context); after the
 * layout call, in place of its `add $0x1c,%esp; test %eax,%eax` (EBP = the
 * text record, EAX = the layout); request_enter (ESI = the object) */
void *nt_enter_fn = (void *)D_ENTER, *nt_ret;
__asm__(".globl _nt_d_dispatch_stub\n_nt_d_dispatch_stub:\n"
        "pushal\n pushfl\n push %edi\n push %ebx\n push %esi\n call _nt_cmd\n add $12,%esp\n popfl\n popal\n"
        "jmp *_nt_dispatch\n"
        ".globl _nt_layout_stub\n_nt_layout_stub:\n"
        "pushal\n push %eax\n push %ebp\n call _nt_layout\n add $8,%esp\n popal\n"
        /* shortcut: one return slot for every thread; LoadText runs on one thread at a time */
        "popl _nt_ret\n add $0x1c,%esp\n test %eax,%eax\n jmp *_nt_ret\n"
        ".globl _nt_enter_stub\n_nt_enter_stub:\n"
        "pushal\n pushfl\n push %esi\n call _nt_enter\n add $4,%esp\n popfl\n popal\n"
        "jmp *_nt_enter_fn\n");
void nt_d_dispatch_stub(void), nt_layout_stub(void), nt_enter_stub(void);

static void redirect_call(unsigned at, void *to) {
    DWORD old;
    VirtualProtect((void *)at, 5, PAGE_EXECUTE_READWRITE, &old);
    *(int *)(at + 1) = (int)to - (int)(at + 5);
    VirtualProtect((void *)at, 5, old, &old);
}

/* Django: its main loop blocks in GetMessageA and, under wine, only goes on
 * while messages come (docs/games/django.md, "Running the original under
 * wine"): hand it a WM_NULL every 10 ms without one */
static BOOL WINAPI nt_get_message(MSG *m, HWND h, UINT lo, UINT hi) {
    while (!PeekMessageA(m, h, lo, hi, PM_REMOVE))
        if (MsgWaitForMultipleObjects(0, NULL, FALSE, 10, QS_ALLINPUT) == WAIT_TIMEOUT) {
            memset(m, 0, sizeof *m);
            return TRUE;
        }
    return m->message != WM_QUIT;
}

/* Django: NSS processes are paused with SuspendThread (00466010), even
 * one waiting for the object mutex (0x52985c). Windows stops such a wait
 * while the thread is suspended; wine can hand the mutex to the suspended
 * thread, and every thread then waits for it (the random hangs under
 * wine). So a thread waiting for that mutex waits in 5 ms steps, and a
 * SuspendThread meant for it is carried out by the thread itself between
 * two steps, without the mutex. */
static HANDLE *obj_mutex;
static CRITICAL_SECTION sus_lock;
static struct { DWORD id; int state; } sus[256]; /* state: 1 waiting, 2 suspend asked for, 3 suspending itself */

static int sus_slot(DWORD id) {
    for (int i = 0; i < 256; i++)
        if (sus[i].id == id) return i;
    for (int i = 0; i < 256; i++)
        if (!sus[i].id) return sus[i].id = id, i;
    return 255; /* shortcut: more than 256 threads share the last slot */
}

static DWORD WINAPI nt_wait(HANDLE h, DWORD ms_) {
    if (!obj_mutex || h != *obj_mutex || ms_ != INFINITE) return sp_WaitForSingleObject(h, ms_);
    DWORD me = GetCurrentThreadId(), r;
    EnterCriticalSection(&sus_lock);
    int k = sus_slot(me);
    sus[k].state = 1;
    LeaveCriticalSection(&sus_lock);
    for (;;) {
        r = WaitForSingleObject(h, 5);
        EnterCriticalSection(&sus_lock);
        if (sus[k].state == 2) {
            if (r != WAIT_TIMEOUT) ReleaseMutex(h);
            sus[k].state = 3;
            LeaveCriticalSection(&sus_lock);
            SuspendThread(GetCurrentThread());
            EnterCriticalSection(&sus_lock);
            sus[k].state = 1;
            LeaveCriticalSection(&sus_lock);
            continue;
        }
        if (r == WAIT_TIMEOUT) {
            LeaveCriticalSection(&sus_lock);
            continue;
        }
        sus[k].state = 0;
        LeaveCriticalSection(&sus_lock);
        return r;
    }
}

static DWORD WINAPI nt_suspend(HANDLE t) {
    DWORD id = GetThreadId(t);
    EnterCriticalSection(&sus_lock);
    int k = sus_slot(id);
    if (sus[k].state == 1) {
        sus[k].state = 2;
        LeaveCriticalSection(&sus_lock);
        return 0;
    }
    LeaveCriticalSection(&sus_lock);
    return SuspendThread(t);
}

static DWORD WINAPI nt_resume(HANDLE t) {
    DWORD id = GetThreadId(t), r;
    for (;;) {
        EnterCriticalSection(&sus_lock);
        int k = sus_slot(id);
        if (sus[k].state == 2) { /* not carried out yet: called off */
            sus[k].state = 1;
            LeaveCriticalSection(&sus_lock);
            return 1;
        }
        int self = sus[k].state == 3;
        LeaveCriticalSection(&sus_lock);
        r = ResumeThread(t);
        if (r || !self) return r;
        Sleep(1); /* it hasn't suspended itself yet */
    }
}

/* point the exe's import of fn at `to` (found by address: a rebuilt import
 * table may have no names) */
static void hook_import(const char *dll, const char *fn, void *to) {
    unsigned char *b = (unsigned char *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS *nt = (IMAGE_NT_HEADERS *)(b + ((IMAGE_DOS_HEADER *)b)->e_lfanew);
    IMAGE_IMPORT_DESCRIPTOR *d =
        (IMAGE_IMPORT_DESCRIPTOR *)(b + nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress);
    DWORD real = (DWORD)GetProcAddress(GetModuleHandleA(dll), fn);
    for (; real && d->Name; d++)
        for (IMAGE_THUNK_DATA *a = (IMAGE_THUNK_DATA *)(b + d->FirstThunk); a->u1.Function; a++)
            if (a->u1.Function == real) {
                DWORD old;
                VirtualProtect(&a->u1.Function, 4, PAGE_READWRITE, &old);
                a->u1.Function = (DWORD)to;
                VirtualProtect(&a->u1.Function, 4, old, &old);
            }
}

static void install(void) {
    const char *p = getenv("NTRACE_OUT");
    out = fopen(p ? p : "C:\\ntrace.jsonl", "w");
    if (!out) return;
    InitializeCriticalSection(&lock);
    if ((p = getenv("NTRACE_PAGES"))) max_pages = atol(p);
    if ((p = getenv("NTRACE_SHOT_REQ"))) lstrcpynA(shot_req, p, sizeof shot_req);
    int django = !memcmp((void *)D_AFTER_LAYOUT, "\x83\xc4\x1c\x85\xc0", 5) && *(unsigned char *)D_CALL_DISPATCH == 0xe8 &&
                 *(unsigned char *)D_CALL_ENTER1 == 0xe8 && *(unsigned char *)D_CALL_ENTER2 == 0xe8;
    fprintf(out, "{\"t\":\"meta\",\"schema\":1,\"engine\":\"%s (ntrace)\",\"max_pages\":%ld,\"speed\":%ld}\n",
            django ? "Django.exe" : "Muramasa.exe", max_pages, (p = getenv("TRACE_SPEED")) && atol(p) > 1 ? atol(p) : 1L);
    if (django) {
        file_base = 0x22c, file_size = 0x14c, file_idx = 0x1c, file_text = 0x114;
        hook_import("user32.dll", "GetMessageA", (void *)nt_get_message);
        obj_mutex = (HANDLE *)0x52985c;
        InitializeCriticalSection(&sus_lock);
        hook_import("kernel32.dll", "WaitForSingleObject", (void *)nt_wait);
        hook_import("kernel32.dll", "SuspendThread", (void *)nt_suspend);
        hook_import("kernel32.dll", "ResumeThread", (void *)nt_resume);
        page_th = 0; /* Enter runs where the queued request is carried out: the pages are the story's */
        nt_dispatch = (void *)D_DISPATCH;
        redirect_call(D_CALL_DISPATCH, (void *)nt_d_dispatch_stub);
        redirect_call(D_CALL_ENTER1, (void *)nt_enter_stub);
        redirect_call(D_CALL_ENTER2, (void *)nt_enter_stub);
        DWORD old;
        VirtualProtect((void *)D_AFTER_LAYOUT, 5, PAGE_EXECUTE_READWRITE, &old);
        *(unsigned char *)D_AFTER_LAYOUT = 0xe8; /* call rel32 */
        VirtualProtect((void *)D_AFTER_LAYOUT, 5, old, &old);
        redirect_call(D_AFTER_LAYOUT, (void *)nt_layout_stub);
        speed_install(); /* after nt_wait, which is hooked by the real function's address */
        fflush(out);
        return;
    }
    if (*(unsigned char *)CALL_DISPATCH != 0xe8 || *(unsigned char *)CALL_BLOCK != 0xe8) {
        fputs("{\"t\":\"error\",\"what\":\"not the exe these addresses are for\"}\n", out);
        fflush(out);
        return;
    }
    redirect_call(CALL_DISPATCH, (void *)nt_dispatch_stub);
    redirect_call(CALL_BLOCK, (void *)nt_block_stub);
    speed_install();
    fflush(out);
}

BOOL WINAPI DllMain(HINSTANCE h, DWORD why, LPVOID r) {
    (void)h, (void)r;
    if (why == DLL_PROCESS_ATTACH) install();
    return TRUE;
}
