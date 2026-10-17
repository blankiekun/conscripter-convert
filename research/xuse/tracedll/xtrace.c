/* xtrace.dll: a JSONL trace of 最果てのイマ (Farthest2015.exe) under wine.
 *
 * Injected by xlaunch.exe (same folder). Hooks, by the addresses in
 * research/xuse/names.tsv (this exe only, sha256 in the game doc):
 * - opcode_table 0x4fb270: every handler goes through `stub`, which logs
 *   the instruction (DAT_005194e8 = its 12 bytes: u16 op, u16 sub, i32
 *   arg, u32 source position) and jumps to the real handler;
 * - Sleep in the import table: frame_loop sleeps once a frame, so the
 *   calls count frames ("f");
 * - story_input_poll 0x477d90 (the input check of every wait): turns the
 *   game's auto mode (0x51f82d) on and takes a hyperlink (0x46) the way
 *   its click branch does (0x477ff0-0x47805c).
 * Records ("t"): meta, cmd (one per dispatched instruction), page (one
 * per message, 0x40/0x41, with its text), choice (links offered on a
 * page and the one taken). "at" = "<script id>:<instruction number>".
 * Environment (wine passes it through):
 *   XTRACE_OUT      Windows path of the JSONL (default C:\xtrace.jsonl)
 *   XTRACE_CHOICES  link label-table indices to take, comma-separated;
 *                   unset: the first link of each page; "none": none
 *   XTRACE_PAGES    exit the game after this many pages (0: never)
 *   XTRACE_NOAUTO   set: leave auto mode alone
 */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define OPCODE_TABLE ((void **)0x4fb270)
#define N_OPS 0x9e
#define CUR_IP (*(unsigned char **)0x5194e8)
#define CUR_SCRIPT (*(unsigned char **)0x5194e4) /* +0x28 code, +0x2c pool */
#define CUR_SCENE (*(unsigned char **)0x51f868)  /* +0x3c script id */
#define AUTO_MODE (*(volatile unsigned char *)0x51f82d)
#define POLL 0x477d90
#define TAKE_LINK 0x470cc0   /* eax = label index; cdecl (task, link word 0) */
#define LINKS_CLEAR 0x49d740 /* eax = 0x51fba0, the link list */

static FILE *out;
static LARGE_INTEGER t0, freq;
static void *orig[N_OPS];
void *xt_jump; /* the handler `stub` jumps to */
static volatile long frames;
static long pages, max_pages;
static int choices[256], n_choices = -1; /* -1: take the first */
static int taken[1024], n_taken;
static struct { int label, extra, row, col, len; } fresh[32], page[32];
static int n_fresh, n_page, last_op = -1;
static unsigned char *last_ip;
static unsigned char poll_tramp[16];
static VOID (WINAPI *real_sleep)(DWORD);

static long ms(void) {
    LARGE_INTEGER t;
    QueryPerformanceCounter(&t);
    return (long)((t.QuadPart - t0.QuadPart) * 1000 / freq.QuadPart);
}

static int script_id(void) { return CUR_SCENE ? *(int *)(CUR_SCENE + 0x3c) : -1; }
static int ins_no(void) {
    return CUR_SCRIPT && CUR_IP ? (int)(CUR_IP - *(unsigned char **)(CUR_SCRIPT + 0x28)) / 12 : -1;
}
static unsigned char *pool(int off) {
    return CUR_SCRIPT ? *(unsigned char **)(CUR_SCRIPT + 0x2c) + off : NULL;
}

/* Shift-JIS (XOR 0x53, as the pool stores it) -> JSON-escaped UTF-8 */
static void put_str(const unsigned char *s, int n) {
    char sj[4096];
    WCHAR w[4096];
    char u[12288];
    if (n > (int)sizeof sj) n = sizeof sj;
    for (int i = 0; i < n; i++) sj[i] = s[i] ^ 0x53;
    int nw = MultiByteToWideChar(932, 0, sj, n, w, 4096);
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

static void log_page(const unsigned char *ip) {
    /* record: u32 id, u32 size, u32 line, u32 segment count, then strings */
    const unsigned char *r = pool(*(int *)(ip + 4));
    if (!r) return;
    int n = *(int *)(r + 12);
    const unsigned char *p = r + 16;
    fprintf(out, "{\"t\":\"page\",\"pg\":%ld,\"ms\":%ld,\"f\":%ld,\"at\":\"%d:%d\",\"voice\":%d,\"line\":%d,\"segs\":[",
            ++pages, ms(), frames, script_id(), ins_no(), *(unsigned short *)ip == 0x41 ? *(int *)r : -1,
            *(int *)(r + 8));
    for (int i = 0; i < n && i < 64; i++) {
        int len = *(int *)p;
        if (i) fputc(',', out);
        put_str(p + 4, len);
        p += 4 + len;
    }
    fputs("]}\n", out);
}

/* called by stub with every register saved; picks the real handler */
void __cdecl xt_cmd(void) {
    unsigned char *ip = CUR_IP;
    int op = ip ? *(unsigned short *)ip : -1;
    xt_jump = op >= 0 && op < N_OPS ? orig[op] : NULL;
    if (!ip || ip == last_ip) return; /* a handler run again for the same instruction */
    last_ip = ip;
    fprintf(out, "{\"t\":\"cmd\",\"pg\":%ld,\"ms\":%ld,\"f\":%ld,\"at\":\"%d:%d\",\"op\":\"0x%02x\",\"sub\":%u,\"arg\":%d,\"src\":%u}\n",
            pages, ms(), frames, script_id(), ins_no(), op, *(unsigned short *)(ip + 2), *(int *)(ip + 4),
            *(unsigned *)(ip + 8));
    if (op == 0x46 && n_fresh < 32) {
        /* record: u16 row, u16 column, u16 length, u32 style, i32 label, u32 (link word 0) */
        const unsigned char *r = pool(*(int *)(ip + 4));
        if (last_op != 0x46) n_fresh = 0;
        fresh[n_fresh].row = *(unsigned short *)r;
        fresh[n_fresh].col = *(unsigned short *)(r + 2);
        fresh[n_fresh].len = *(unsigned short *)(r + 4);
        fresh[n_fresh].label = *(int *)(r + 10);
        fresh[n_fresh++].extra = *(int *)(r + 14);
    }
    if (op == 0x40 || op == 0x41) {
        log_page(ip);
        memcpy(page, fresh, sizeof page); /* the links belong to this message */
        n_page = last_op == 0x46 ? n_fresh : 0;
        n_fresh = 0;
        if (max_pages && pages >= max_pages) {
            fflush(out);
            TerminateProcess(GetCurrentProcess(), 0);
        }
    }
    last_op = op;
    fflush(out);
}

/* every handler entry: save all, log, jump to the real handler (ESI = task) */
__asm__(".globl _xt_stub\n_xt_stub:\n"
        "pushal\n pushfl\n call _xt_cmd\n popfl\n popal\n"
        "jmp *_xt_jump\n");
void xt_stub(void);

static int was_taken(int l) {
    for (int i = 0; i < n_taken; i++)
        if (taken[i] == l) return 1;
    return 0;
}

static int pick(void) {
    for (int i = 0; i < n_page; i++) {
        if (was_taken(page[i].label)) continue;
        if (n_choices < 0) return i == 0 ? 0 : -1;
        for (int k = 0; k < n_choices; k++)
            if (choices[k] == page[i].label) return i;
    }
    return -1;
}

typedef unsigned (__cdecl *poll_fn)(unsigned *, char, char *, unsigned);

/* story_input_poll(result, ?, task, resume stub) */
unsigned __cdecl xt_poll(unsigned *res, char a2, char *task, unsigned a4) {
    if (!getenv("XTRACE_NOAUTO")) AUTO_MODE = 1;
    if (n_page) {
        int k = pick();
        fprintf(out, "{\"t\":\"choice\",\"pg\":%ld,\"ms\":%ld,\"f\":%ld,\"kind\":\"link\",\"options\":[", pages, ms(),
                frames);
        for (int i = 0; i < n_page; i++)
            fprintf(out, "%s{\"label\":%d,\"row\":%d,\"col\":%d,\"len\":%d}", i ? "," : "", page[i].label,
                    page[i].row, page[i].col, page[i].len);
        fprintf(out, "],\"picked\":%d}\n", k + 1);
        fflush(out);
        int label = k >= 0 ? page[k].label : 0, extra = k >= 0 ? page[k].extra : 0;
        n_page = 0;
        if (k >= 0) {
            unsigned char ok;
            taken[n_taken++ % 1024] = label;
            __asm__ volatile("push %2\n push %1\n call *%3\n add $8,%%esp"
                             : "=a"(ok) : "r"(task), "r"(extra), "r"(TAKE_LINK), "0"(label)
                             : "ecx", "edx", "memory", "cc");
            if (ok) { /* the rest of the click branch, without the link object's own call */
                __asm__ volatile("call *%1" : : "a"(0x51fba0), "r"(LINKS_CLEAR) : "ecx", "edx", "memory", "cc");
                *(unsigned *)(*(unsigned char **)0x51e794 + 0x10) &= ~2u;
                *(unsigned *)(task + 0x14) = 0x40aac0;
                *(unsigned *)(task + 0x18) = 0x46ee20;
                *res = 2;
                return 0x40aa01;
            }
        }
    }
    return ((poll_fn)(void *)poll_tramp)(res, a2, task, a4);
}

static VOID WINAPI xt_sleep(DWORD n) {
    frames++;
    real_sleep(n);
}

static void hook_iat(const char *dll, const char *fn, void *to, void **save) {
    unsigned char *b = (unsigned char *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS *nt = (IMAGE_NT_HEADERS *)(b + ((IMAGE_DOS_HEADER *)b)->e_lfanew);
    IMAGE_IMPORT_DESCRIPTOR *d =
        (IMAGE_IMPORT_DESCRIPTOR *)(b + nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress);
    for (; d->Name; d++) {
        if (_stricmp((char *)b + d->Name, dll)) continue;
        IMAGE_THUNK_DATA *n = (IMAGE_THUNK_DATA *)(b + d->OriginalFirstThunk), *a = (IMAGE_THUNK_DATA *)(b + d->FirstThunk);
        for (; n->u1.AddressOfData; n++, a++) {
            if (IMAGE_SNAP_BY_ORDINAL(n->u1.Ordinal) || strcmp((char *)((IMAGE_IMPORT_BY_NAME *)(b + n->u1.AddressOfData))->Name, fn)) continue;
            DWORD old;
            VirtualProtect(&a->u1.Function, 4, PAGE_READWRITE, &old);
            *save = (void *)a->u1.Function;
            a->u1.Function = (DWORD)to;
            VirtualProtect(&a->u1.Function, 4, old, &old);
        }
    }
}

static void install(void) {
    const char *p = getenv("XTRACE_OUT");
    out = fopen(p ? p : "C:\\xtrace.jsonl", "w");
    if (!out) return;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);
    if ((p = getenv("XTRACE_PAGES"))) max_pages = atol(p);
    const char *ch = getenv("XTRACE_CHOICES");
    if (ch) {
        n_choices = 0;
        for (char *s = (char *)ch; *s && n_choices < 256; s++)
            if (*s >= '0' && *s <= '9') choices[n_choices++] = strtol(s, &s, 10), s--;
    }
    fprintf(out, "{\"t\":\"meta\",\"schema\":1,\"engine\":\"Farthest2015.exe (xtrace)\",\"choices\":\"%s\",\"max_pages\":%ld}\n",
            ch ? ch : "first", max_pages);
    DWORD old;
    VirtualProtect(OPCODE_TABLE, N_OPS * 4, PAGE_READWRITE, &old);
    for (int i = 0; i < N_OPS; i++)
        if ((orig[i] = OPCODE_TABLE[i])) OPCODE_TABLE[i] = (void *)xt_stub;
    VirtualProtect(OPCODE_TABLE, N_OPS * 4, old, &old);
    hook_iat("KERNEL32.dll", "Sleep", (void *)xt_sleep, (void **)&real_sleep);
    /* poll: its first 7 bytes (sub esp,0x24; push ebx/ebp/esi/edi) move to the trampoline */
    VirtualProtect(poll_tramp, sizeof poll_tramp, PAGE_EXECUTE_READWRITE, &old);
    memcpy(poll_tramp, (void *)POLL, 7);
    poll_tramp[7] = 0xe9;
    *(int *)(poll_tramp + 8) = (POLL + 7) - (int)(poll_tramp + 12);
    VirtualProtect((void *)POLL, 7, PAGE_EXECUTE_READWRITE, &old);
    *(unsigned char *)POLL = 0xe9;
    *(int *)(POLL + 1) = (int)xt_poll - (POLL + 5);
    *(unsigned short *)(POLL + 5) = 0x9090;
    VirtualProtect((void *)POLL, 7, old, &old);
    fflush(out);
}

BOOL WINAPI DllMain(HINSTANCE h, DWORD why, LPVOID r) {
    (void)h, (void)r;
    if (why == DLL_PROCESS_ATTACH) install();
    return TRUE;
}
