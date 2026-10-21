/* fctrace.dll: a generic JSONL trace of any Windows game under wine, at the
 * API level, for a first contact with a new engine (docs/new-game-reference.md, "First contact";
 * run it with fc.sh). Injected by research/xuse/tracedll/xlaunch.exe.
 *
 * It points the import tables of every module but ntdll/kernel32/
 * kernelbase/win32u and the DLLs in its own folder (fc.sh puts other
 * injected trace DLLs there) at its hooks (by the imported name, else by the real
 * function's address: imports by ordinal and rebuilt tables work), again
 * each second for modules loaded later, and makes GetProcAddress hand out
 * the hooks too. A call made inside a hooked call is not logged again.
 * COM objects (DirectSound) get their vtable slots patched in place.
 *
 * Records ("t"; "ms" = real ms since the DLL loaded, "th" = OS thread id;
 * "stack" on open, dsbuf, play and the first 5 and every 500th present:
 * the exe's call sites on the stack, innermost first, at the exe's
 * preferred addresses: the function holding one is the last functions.tsv
 * address below it):
 *   meta     exe, bitness, the hooks file's breakpoints
 *   open     a file opened (CreateFile): "name", "w" when for writing,
 *            "err" when it failed (2: not found)
 *   read / write  file I/O: "name", "off", "n"; contiguous calls on one
 *            handle are folded into one record
 *   find / attr   FindFirstFile / GetFileAttributes patterns (probes)
 *   present  a frame shown: "n" (count), "api" (SwapBuffers, wglSwapBuffers,
 *            or a GDI blit to a window: BitBlt, StretchBlt, StretchDIBits,
 *            SetDIBitsToDevice). Under wine, D3D8/9/11 and DirectDraw end in
 *            SwapBuffers (wined3d's GL renderer), so they count there
 *   dsbuf    a DirectSound buffer made: "id", "bytes", "hz", "ch", "bits",
 *            "src" (the file read last before it: a guess at its sound)
 *   play / stop / vol / pos  DirectSound buffer Play ("loop"), Stop,
 *            SetVolume ("db100", 1/100 dB; repeats of one value left out),
 *            SetCurrentPosition
 *   wave     waveOutOpen / waveOutReset; mci: mciSendString's command
 *   com      CoCreateInstance's CLSID (DirectShow, XAudio2, ...)
 *   window   CreateWindowEx: class, title, size; msgbox: MessageBox text
 *   input    a mouse button, wheel or key message dispatched (DispatchMessage)
 *   timer    SetTimer / timeSetEvent and their period
 *   sec      once a second, the calls of the game's own modules (the exe
 *            and DLLs in its folder) in that second: sleep (count, sleep_ms
 *            sum, min, max), tgt (timeGetTime), qpc, gtc (GetTickCount),
 *            wout (waveOutWrite), present, probe (find/attr of a name
 *            logged before: each name is logged once), and "hits" of
 *            count breakpoints
 *   bp       a breakpoint from the hooks file hit: "name", "r" (32-bit: eax
 *            ecx edx ebx esi edi ebp esp; 64-bit: rcx rdx r8 r9 rax rbx rsi
 *            rdi), "s" (4 stack words above the return address), "str"
 *            (those of them that point at a cp932 string)
 *   shot     a screenshot asked for: "n" (fc.sh saves shots/<n>.webp or
 *            leaves it out when the screen hasn't changed)
 * Environment:
 *   FC_OUT    Windows path of the JSONL (default C:\fctrace.jsonl)
 *   FC_HOOKS  a file of `<hex address> <name> [once|count|every]` lines
 *             (tab-separated; a names.tsv works, other third columns mean
 *             once): once logs the first hit, every each hit (the first
 *             200, then counts), count only counts (in "sec"). Addresses
 *             are the exe's (rebased when ASLR moved it)
 *   FC_SHOT_EVERY  ask for a screenshot every N presents (0: none)
 *   FC_SHOT_REQ    Windows path of the request file: the DLL writes n into
 *             it and waits (up to 2 s) for fc.sh to take the shot and
 *             delete it, so the shot is the frame just shown
 */
#include <windows.h>
#include <mmsystem.h>
#include <tlhelp32.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN64
#define PC(c) ((c)->Rip)
#define BITS 64
#else
#define PC(c) ((c)->Eip)
#define BITS 32
#endif

static FILE *out;
static CRITICAL_SECTION lock;
static LARGE_INTEGER q0, qf;
static DWORD busy_tls;
static HMODULE self;
static char game_dir[MAX_PATH], self_dir[MAX_PATH]; /* the exe's folder; ours (other injected trace DLLs too) */

static long ms(void) {
    LARGE_INTEGER t;
    QueryPerformanceCounter(&t);
    return (long)((t.QuadPart - q0.QuadPart) * 1000 / qf.QuadPart);
}

/* inside a hooked call already? (keeps the caller's last error) */
static int enter(void) {
    DWORD e = GetLastError();
    int top = !TlsGetValue(busy_tls);
    if (top) TlsSetValue(busy_tls, (void *)1);
    SetLastError(e);
    return top;
}
static void leave(int top) {
    DWORD e = GetLastError();
    if (top) TlsSetValue(busy_tls, NULL);
    SetLastError(e);
}

/* the game's own code: the exe and the DLLs in its folder */
static struct { char *lo, *hi; } game_mods[64];
static int n_game_mods;
static int from_game(void *ret) {
    for (int i = 0; i < n_game_mods; i++)
        if ((char *)ret >= game_mods[i].lo && (char *)ret < game_mods[i].hi) return 1;
    return 0;
}

/* a JSON string from cp932 (or UTF-16 with wide) bytes */
static void put_str(const void *s, int n, int wide) {
    static WCHAR w[4096];
    static char u[3 * 4096];
    int nw = wide ? (n > 4096 ? 4096 : n) : MultiByteToWideChar(932, 0, s, n, w, 4096);
    if (wide) memcpy(w, s, nw * 2);
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
static void put_a(const char *s) { put_str(s, s ? (int)strlen(s) : 0, 0); }
static void put_w(const WCHAR *s) { put_str(s, s ? (int)wcslen(s) : 0, 1); }

/* the exe's call sites on this thread's stack, innermost first: words that
 * point into the exe just past a call instruction, as the exe's preferred
 * addresses (Ghidra's). shortcut: a stale return address left on the stack
 * can show up too; the first ones are the reliable part */
static char *exe_lo, *exe_hi;
static ULONG_PTR exe_slide;
static void put_stack(void) {
    ULONG_PTR *p = (ULONG_PTR *)&p, *top = (ULONG_PTR *)((NT_TIB *)NtCurrentTeb())->StackBase;
    int k = 0;
    fputs(",\"stack\":[", out);
    for (int i = 0; i < 4096 && p + i < top && k < 6; i++) {
        unsigned char *c = (unsigned char *)p[i];
        if (c < (unsigned char *)exe_lo + 0x1000 || c >= (unsigned char *)exe_hi) continue;
        if (c[-5] == 0xe8 || (c[-2] == 0xff && (c[-1] & 0x38) == 0x10) || (c[-3] == 0xff && (c[-2] & 0x38) == 0x10) ||
            (c[-6] == 0xff && (c[-5] & 0x38) == 0x10) || (c[-7] == 0xff && (c[-6] & 0x38) == 0x10))
            fprintf(out, "%s\"%08llx\"", k++ ? "," : "", (unsigned long long)((ULONG_PTR)c - exe_slide));
    }
    fputc(']', out);
}

static void head(const char *t) { fprintf(out, "{\"t\":\"%s\",\"ms\":%ld,\"th\":%lu", t, ms(), GetCurrentThreadId()); }
static void end_rec(void) { fputs("}\n", out); fflush(out); }

/* ---- files ---- */
static struct { HANDLE h; char name[260]; } files[1024];
static int n_files;
static char last_read[300]; /* "name@off" of the latest read: a sound buffer's guessed source */
static struct { HANDLE h; int w; LONGLONG off, n; } pend; /* the read/write being folded */

static const char *file_name(HANDLE h) {
    for (int i = 0; i < n_files; i++)
        if (files[i].h == h) return files[i].name;
    return NULL;
}
static void flush_io(void) {
    if (!pend.h) return;
    head(pend.w ? "write" : "read");
    fputs(",\"name\":", out);
    put_a(file_name(pend.h));
    fprintf(out, ",\"off\":%lld,\"n\":%lld", pend.off, pend.n);
    end_rec();
    pend.h = NULL;
}
/* every other record goes after the folded I/O */
#define REC(t) (flush_io(), head(t))

static void add_file(HANDLE h, const char *utf8_or_cp932) {
    int i;
    for (i = 0; i < n_files && files[i].h != h; i++) {}
    if (i == n_files) {
        if (n_files == 1024) i = (int)((ULONG_PTR)h / 4 % 1024); /* shortcut: a full table overwrites a slot */
        else n_files++;
    }
    files[i].h = h;
    lstrcpynA(files[i].name, utf8_or_cp932, sizeof files[i].name);
}

static void io(HANDLE h, int w, DWORD n) {
    const char *name = file_name(h);
    if (!name || !n || lock.OwningThread == (HANDLE)(ULONG_PTR)GetCurrentThreadId()) return; /* our own log's writes */
    LARGE_INTEGER z = {0}, cur;
    if (!SetFilePointerEx(h, z, &cur, FILE_CURRENT)) return;
    LONGLONG off = cur.QuadPart - n; /* called after the transfer */
    EnterCriticalSection(&lock);
    if (pend.h == h && pend.w == w && pend.off + pend.n == off) pend.n += n;
    else flush_io(), pend.h = h, pend.w = w, pend.off = off, pend.n = n;
    if (!w) _snprintf(last_read, sizeof last_read, "%s@%lld", name, off);
    LeaveCriticalSection(&lock);
}

/* a probe (find/attr) of a name seen before: only counted ("probe" in "sec") */
static volatile LONG c_probe;
static int probe_seen(const void *n, int wide) {
    static unsigned seen[4096];
    unsigned h = 2166136261u;
    for (const unsigned char *p = n; wide ? (p[0] | p[1]) : *p; p += wide ? 2 : 1) h = (h ^ *p) * 16777619u;
    h |= 1;
    unsigned *slot = &seen[h % 4096]; /* shortcut: a collision logs a name again */
    if (*slot == h) return InterlockedIncrement(&c_probe), 1;
    *slot = h;
    return 0;
}

/* ---- per-second counts ---- */
static volatile LONG c_sleep, c_sleep_ms, c_tgt, c_qpc, c_gtc, c_wout, c_present, sleep_min = -1, sleep_max;
static volatile LONG presents;
static long shot_every;
static char shot_req[MAX_PATH];

static void present(const char *api) {
    LONG n = InterlockedIncrement(&presents);
    InterlockedIncrement(&c_present);
    EnterCriticalSection(&lock);
    REC("present");
    fprintf(out, ",\"n\":%ld,\"api\":\"%s\"", n, api);
    if (n <= 5 || n % 500 == 0) put_stack();
    end_rec();
    LeaveCriticalSection(&lock);
    if (shot_every && *shot_req && n % shot_every == 0) {
        long k = n / shot_every;
        EnterCriticalSection(&lock);
        REC("shot");
        fprintf(out, ",\"n\":%ld", k);
        end_rec();
        LeaveCriticalSection(&lock);
        FILE *f = fopen(shot_req, "w");
        if (f) {
            fprintf(f, "%ld\n", k);
            fclose(f);
            for (int i = 0; i < 400 && GetFileAttributesA(shot_req) != INVALID_FILE_ATTRIBUTES; i++) Sleep(5);
        }
    }
}

/* ---- hooks ---- */
#define H(ret, name, args) static ret (WINAPI *o_##name) args; static ret WINAPI h_##name args

H(HANDLE, CreateFileA, (LPCSTR n, DWORD a, DWORD s, LPSECURITY_ATTRIBUTES sa, DWORD c, DWORD f, HANDLE t)) {
    int top = enter();
    HANDLE h = o_CreateFileA(n, a, s, sa, c, f, t);
    DWORD e = GetLastError();
    if (top && n && _strnicmp(n, self_dir, strlen(self_dir))) { /* not our or another trace DLL's log */
        EnterCriticalSection(&lock);
        REC("open");
        fputs(",\"name\":", out);
        put_a(n);
        if (a & GENERIC_WRITE) fputs(",\"w\":1", out);
        if (h == INVALID_HANDLE_VALUE) fprintf(out, ",\"err\":%lu", e);
        else add_file(h, n);
        put_stack();
        end_rec();
        LeaveCriticalSection(&lock);
    }
    leave(top);
    SetLastError(e);
    return h;
}
H(HANDLE, CreateFileW, (LPCWSTR n, DWORD a, DWORD s, LPSECURITY_ATTRIBUTES sa, DWORD c, DWORD f, HANDLE t)) {
    int top = enter();
    HANDLE h = o_CreateFileW(n, a, s, sa, c, f, t);
    DWORD e = GetLastError();
    char nm[260];
    WideCharToMultiByte(932, 0, n ? n : L"", -1, nm, sizeof nm, NULL, NULL);
    nm[259] = 0;
    if (top && n && _strnicmp(nm, self_dir, strlen(self_dir))) {
        EnterCriticalSection(&lock);
        REC("open");
        fputs(",\"name\":", out);
        put_w(n);
        if (a & GENERIC_WRITE) fputs(",\"w\":1", out);
        if (h == INVALID_HANDLE_VALUE) fprintf(out, ",\"err\":%lu", e);
        else add_file(h, nm);
        put_stack();
        end_rec();
        LeaveCriticalSection(&lock);
    }
    leave(top);
    SetLastError(e);
    return h;
}
H(BOOL, ReadFile, (HANDLE h, LPVOID b, DWORD n, LPDWORD got, LPOVERLAPPED ov)) {
    int top = enter();
    DWORD g = 0;
    BOOL r = o_ReadFile(h, b, n, got ? got : &g, ov);
    DWORD e = GetLastError();
    if (top && r && !ov) io(h, 0, got ? *got : g);
    leave(top);
    SetLastError(e);
    return r;
}
H(BOOL, WriteFile, (HANDLE h, LPCVOID b, DWORD n, LPDWORD put, LPOVERLAPPED ov)) {
    int top = enter();
    DWORD g = 0;
    BOOL r = o_WriteFile(h, b, n, put ? put : &g, ov);
    DWORD e = GetLastError();
    if (top && r && !ov) io(h, 1, put ? *put : g);
    leave(top);
    SetLastError(e);
    return r;
}
#define PROBE(name, T, put, ret, fail, call)                                  \
    H(ret, name, (T n, void *x)) {                                            \
        int top = enter();                                                    \
        ret r = call;                                                         \
        DWORD e = GetLastError();                                             \
        if (top && n && !probe_seen(n, sizeof *n == 2)) {                     \
            EnterCriticalSection(&lock);                                      \
            REC(#name[0] == 'F' ? "find" : "attr");                           \
            fputs(",\"name\":", out);                                         \
            put(n);                                                           \
            if (r == fail) fprintf(out, ",\"err\":%lu", e);                   \
            end_rec();                                                        \
            LeaveCriticalSection(&lock);                                      \
        }                                                                     \
        leave(top);                                                           \
        SetLastError(e);                                                      \
        return r;                                                             \
    }
PROBE(FindFirstFileA, LPCSTR, put_a, HANDLE, INVALID_HANDLE_VALUE, o_FindFirstFileA(n, x))
PROBE(FindFirstFileW, LPCWSTR, put_w, HANDLE, INVALID_HANDLE_VALUE, o_FindFirstFileW(n, x))
static DWORD (WINAPI *o_GetFileAttributesA)(LPCSTR), (WINAPI *o_GetFileAttributesW)(LPCWSTR);
static DWORD WINAPI h_GetFileAttributesA(LPCSTR n) {
    int top = enter();
    DWORD r = o_GetFileAttributesA(n), e = GetLastError();
    if (top && n && !probe_seen(n, 0)) {
        EnterCriticalSection(&lock);
        REC("attr");
        fputs(",\"name\":", out);
        put_a(n);
        if (r == INVALID_FILE_ATTRIBUTES) fprintf(out, ",\"err\":%lu", e);
        end_rec();
        LeaveCriticalSection(&lock);
    }
    leave(top);
    SetLastError(e);
    return r;
}
static DWORD WINAPI h_GetFileAttributesW(LPCWSTR n) {
    int top = enter();
    DWORD r = o_GetFileAttributesW(n), e = GetLastError();
    if (top && n && !probe_seen(n, 1)) {
        EnterCriticalSection(&lock);
        REC("attr");
        fputs(",\"name\":", out);
        put_w(n);
        if (r == INVALID_FILE_ATTRIBUTES) fprintf(out, ",\"err\":%lu", e);
        end_rec();
        LeaveCriticalSection(&lock);
    }
    leave(top);
    SetLastError(e);
    return r;
}

/* clocks and sleeps: counted when the game's own code calls them */
static void slept(DWORD t) {
    InterlockedIncrement(&c_sleep);
    InterlockedExchangeAdd(&c_sleep_ms, t);
    if (sleep_min < 0 || (LONG)t < sleep_min) sleep_min = t;
    if ((LONG)t > sleep_max) sleep_max = t;
}
static VOID (WINAPI *o_Sleep)(DWORD);
static VOID WINAPI h_Sleep(DWORD t) { if (from_game(__builtin_return_address(0))) slept(t); o_Sleep(t); }
static DWORD (WINAPI *o_SleepEx)(DWORD, BOOL);
static DWORD WINAPI h_SleepEx(DWORD t, BOOL a) { if (from_game(__builtin_return_address(0))) slept(t); return o_SleepEx(t, a); }
static DWORD (WINAPI *o_timeGetTime)(void);
static DWORD WINAPI h_timeGetTime(void) { if (from_game(__builtin_return_address(0))) InterlockedIncrement(&c_tgt); return o_timeGetTime(); }
static DWORD (WINAPI *o_GetTickCount)(void);
static DWORD WINAPI h_GetTickCount(void) { if (from_game(__builtin_return_address(0))) InterlockedIncrement(&c_gtc); return o_GetTickCount(); }
static BOOL (WINAPI *o_QueryPerformanceCounter)(LARGE_INTEGER *);
static BOOL WINAPI h_QueryPerformanceCounter(LARGE_INTEGER *t) {
    if (from_game(__builtin_return_address(0))) InterlockedIncrement(&c_qpc);
    return o_QueryPerformanceCounter(t);
}
static void timer_rec(const char *api, UINT period) {
    EnterCriticalSection(&lock);
    REC("timer");
    fprintf(out, ",\"api\":\"%s\",\"period\":%u", api, period);
    end_rec();
    LeaveCriticalSection(&lock);
}
H(MMRESULT, timeSetEvent, (UINT d, UINT r, LPTIMECALLBACK cb, DWORD_PTR u, UINT f)) {
    if (from_game(__builtin_return_address(0))) timer_rec("timeSetEvent", d);
    return o_timeSetEvent(d, r, cb, u, f);
}
H(UINT_PTR, SetTimer, (HWND w, UINT_PTR id, UINT d, TIMERPROC p)) {
    if (from_game(__builtin_return_address(0))) timer_rec("SetTimer", d);
    return o_SetTimer(w, id, d, p);
}

/* presents */
H(BOOL, SwapBuffers, (HDC dc)) {
    int top = enter();
    BOOL r = o_SwapBuffers(dc);
    if (top) present("SwapBuffers");
    leave(top);
    return r;
}
H(BOOL, wglSwapBuffers, (HDC dc)) {
    int top = enter();
    BOOL r = o_wglSwapBuffers(dc);
    if (top) present("wglSwapBuffers");
    leave(top);
    return r;
}
static int window_dc(HDC dc) { return GetObjectType(dc) == OBJ_DC && WindowFromDC(dc); }
H(BOOL, BitBlt, (HDC d, int x, int y, int w, int h, HDC s, int sx, int sy, DWORD op)) {
    int top = enter();
    BOOL r = o_BitBlt(d, x, y, w, h, s, sx, sy, op);
    if (top && window_dc(d)) present("BitBlt");
    leave(top);
    return r;
}
H(BOOL, StretchBlt, (HDC d, int x, int y, int w, int h, HDC s, int sx, int sy, int sw, int sh, DWORD op)) {
    int top = enter();
    BOOL r = o_StretchBlt(d, x, y, w, h, s, sx, sy, sw, sh, op);
    if (top && window_dc(d)) present("StretchBlt");
    leave(top);
    return r;
}
H(int, StretchDIBits, (HDC d, int x, int y, int w, int h, int sx, int sy, int sw, int sh, const void *b, const BITMAPINFO *bi, UINT u, DWORD op)) {
    int top = enter();
    int r = o_StretchDIBits(d, x, y, w, h, sx, sy, sw, sh, b, bi, u, op);
    if (top && window_dc(d)) present("StretchDIBits");
    leave(top);
    return r;
}
H(int, SetDIBitsToDevice, (HDC d, int x, int y, DWORD w, DWORD h, int sx, int sy, UINT a, UINT n, const void *b, const BITMAPINFO *bi, UINT u)) {
    int top = enter();
    int r = o_SetDIBitsToDevice(d, x, y, w, h, sx, sy, a, n, b, bi, u);
    if (top && window_dc(d)) present("SetDIBitsToDevice");
    leave(top);
    return r;
}

/* windows and input */
static const char *input_name(UINT m) {
    switch (m) {
    case WM_LBUTTONDOWN: return "ldown"; case WM_LBUTTONUP: return "lup";
    case WM_RBUTTONDOWN: return "rdown"; case WM_RBUTTONUP: return "rup";
    case WM_MBUTTONDOWN: return "mdown"; case WM_MOUSEWHEEL: return "wheel";
    case WM_KEYDOWN: return "keydown"; case WM_KEYUP: return "keyup";
    case WM_SYSKEYDOWN: return "syskeydown"; case WM_CHAR: return "char";
    }
    return NULL;
}
static void input(const MSG *m) {
    const char *n = m ? input_name(m->message) : NULL;
    if (!n) return;
    EnterCriticalSection(&lock);
    REC("input");
    fprintf(out, ",\"msg\":\"%s\"", n);
    if (m->message >= WM_MOUSEFIRST) fprintf(out, ",\"x\":%d,\"y\":%d", (short)LOWORD(m->lParam), (short)HIWORD(m->lParam));
    if (m->message == WM_MOUSEWHEEL) fprintf(out, ",\"d\":%d", (short)HIWORD(m->wParam));
    if (m->message < WM_MOUSEFIRST) fprintf(out, ",\"vk\":%u", (unsigned)m->wParam);
    end_rec();
    LeaveCriticalSection(&lock);
}
H(LRESULT, DispatchMessageA, (const MSG *m)) { input(m); return o_DispatchMessageA(m); }
H(LRESULT, DispatchMessageW, (const MSG *m)) { input(m); return o_DispatchMessageW(m); }
H(HWND, CreateWindowExA, (DWORD xs, LPCSTR c, LPCSTR t, DWORD s, int x, int y, int w, int h, HWND p, HMENU mn, HINSTANCE i, LPVOID l)) {
    HWND r = o_CreateWindowExA(xs, c, t, s, x, y, w, h, p, mn, i, l);
    if (from_game(__builtin_return_address(0))) {
        EnterCriticalSection(&lock);
        REC("window");
        fputs(",\"class\":", out);
        if ((ULONG_PTR)c > 0xffff) put_a(c); else fprintf(out, "%u", (unsigned)(ULONG_PTR)c);
        fputs(",\"title\":", out);
        put_a(t);
        fprintf(out, ",\"w\":%d,\"h\":%d,\"style\":\"%08lx\"", w, h, s);
        end_rec();
        LeaveCriticalSection(&lock);
    }
    return r;
}
H(HWND, CreateWindowExW, (DWORD xs, LPCWSTR c, LPCWSTR t, DWORD s, int x, int y, int w, int h, HWND p, HMENU mn, HINSTANCE i, LPVOID l)) {
    HWND r = o_CreateWindowExW(xs, c, t, s, x, y, w, h, p, mn, i, l);
    if (from_game(__builtin_return_address(0))) {
        EnterCriticalSection(&lock);
        REC("window");
        fputs(",\"class\":", out);
        if ((ULONG_PTR)c > 0xffff) put_w(c); else fprintf(out, "%u", (unsigned)(ULONG_PTR)c);
        fputs(",\"title\":", out);
        put_w(t);
        fprintf(out, ",\"w\":%d,\"h\":%d,\"style\":\"%08lx\"", w, h, s);
        end_rec();
        LeaveCriticalSection(&lock);
    }
    return r;
}
H(int, MessageBoxA, (HWND w, LPCSTR t, LPCSTR c, UINT f)) {
    EnterCriticalSection(&lock); REC("msgbox"); fputs(",\"text\":", out); put_a(t); fputs(",\"caption\":", out); put_a(c);
    end_rec(); LeaveCriticalSection(&lock);
    return o_MessageBoxA(w, t, c, f);
}
H(int, MessageBoxW, (HWND w, LPCWSTR t, LPCWSTR c, UINT f)) {
    EnterCriticalSection(&lock); REC("msgbox"); fputs(",\"text\":", out); put_w(t); fputs(",\"caption\":", out); put_w(c);
    end_rec(); LeaveCriticalSection(&lock);
    return o_MessageBoxW(w, t, c, f);
}

/* waveOut, MCI, COM */
H(MMRESULT, waveOutOpen, (LPHWAVEOUT h, UINT d, LPCWAVEFORMATEX f, DWORD_PTR cb, DWORD_PTR i, DWORD fl)) {
    int top = enter();
    MMRESULT r = o_waveOutOpen(h, d, f, cb, i, fl);
    if (top && f && !(fl & WAVE_FORMAT_QUERY)) {
        EnterCriticalSection(&lock); REC("wave");
        fprintf(out, ",\"op\":\"open\",\"hz\":%lu,\"ch\":%u,\"bits\":%u,\"tag\":%u,\"src\":", f->nSamplesPerSec, f->nChannels, f->wBitsPerSample, f->wFormatTag);
        put_a(last_read); end_rec(); LeaveCriticalSection(&lock);
    }
    leave(top);
    return r;
}
H(MMRESULT, waveOutWrite, (HWAVEOUT h, LPWAVEHDR w, UINT n)) {
    if (from_game(__builtin_return_address(0))) InterlockedIncrement(&c_wout);
    return o_waveOutWrite(h, w, n);
}
H(MMRESULT, waveOutReset, (HWAVEOUT h)) {
    EnterCriticalSection(&lock); REC("wave"); fputs(",\"op\":\"reset\"", out); end_rec(); LeaveCriticalSection(&lock);
    return o_waveOutReset(h);
}
H(MCIERROR, mciSendStringA, (LPCSTR c, LPSTR r, UINT n, HWND w)) {
    EnterCriticalSection(&lock); REC("mci"); fputs(",\"cmd\":", out); put_a(c); end_rec(); LeaveCriticalSection(&lock);
    return o_mciSendStringA(c, r, n, w);
}
H(MCIERROR, mciSendStringW, (LPCWSTR c, LPWSTR r, UINT n, HWND w)) {
    EnterCriticalSection(&lock); REC("mci"); fputs(",\"cmd\":", out); put_w(c); end_rec(); LeaveCriticalSection(&lock);
    return o_mciSendStringW(c, r, n, w);
}

/* ---- COM vtables (DirectSound) ---- */
static struct { void **vt; void *orig[24]; } vts[16];
static int n_vts;
static void *vt_orig(void *obj, int i) {
    void **vt = *(void ***)obj;
    for (int k = 0; k < n_vts; k++)
        if (vts[k].vt == vt) return vts[k].orig[i];
    return NULL;
}
/* patch slots idx[] of obj's vtable with to[] (once per vtable) */
static void vt_hook(void *obj, const int *idx, void *const *to, int n) {
    void **vt = *(void ***)obj;
    EnterCriticalSection(&lock);
    int k;
    for (k = 0; k < n_vts && vts[k].vt != vt; k++) {}
    if (k == n_vts && n_vts < 16) {
        n_vts++;
        vts[k].vt = vt;
        for (int j = 0; j < n; j++) {
            DWORD old;
            if (vt[idx[j]] == to[j]) continue;
            vts[k].orig[idx[j]] = vt[idx[j]];
            VirtualProtect(&vt[idx[j]], sizeof(void *), PAGE_READWRITE, &old);
            vt[idx[j]] = to[j];
            VirtualProtect(&vt[idx[j]], sizeof(void *), old, &old);
        }
    }
    LeaveCriticalSection(&lock);
}
static struct { void *buf; LONG vol; } bufs[4096];
static int n_bufs;
static int buf_id(void *b) {
    for (int i = n_bufs - 1; i >= 0; i--)
        if (bufs[i].buf == b) return i;
    return -1;
}
typedef HRESULT(WINAPI *dsb_play_t)(void *, DWORD, DWORD, DWORD);
typedef HRESULT(WINAPI *dsb_1_t)(void *);
typedef HRESULT(WINAPI *dsb_l_t)(void *, LONG);
static void buf_rec(void *b, const char *t) { REC(t); fprintf(out, ",\"id\":%d", buf_id(b)); }
static HRESULT WINAPI dsb_play(void *b, DWORD r, DWORD p, DWORD f) {
    EnterCriticalSection(&lock); buf_rec(b, "play"); if (f & 1) fputs(",\"loop\":1", out); put_stack(); end_rec(); LeaveCriticalSection(&lock);
    return ((dsb_play_t)vt_orig(b, 12))(b, r, p, f);
}
static HRESULT WINAPI dsb_stop(void *b) {
    EnterCriticalSection(&lock); buf_rec(b, "stop"); end_rec(); LeaveCriticalSection(&lock);
    return ((dsb_1_t)vt_orig(b, 18))(b);
}
static HRESULT WINAPI dsb_vol(void *b, LONG v) {
    int i = buf_id(b);
    EnterCriticalSection(&lock);
    if (i < 0 || bufs[i].vol != v) { buf_rec(b, "vol"); fprintf(out, ",\"db100\":%ld", v); end_rec(); if (i >= 0) bufs[i].vol = v; }
    LeaveCriticalSection(&lock);
    return ((dsb_l_t)vt_orig(b, 15))(b, v);
}
static HRESULT WINAPI dsb_pos(void *b, LONG p) {
    EnterCriticalSection(&lock); buf_rec(b, "pos"); fprintf(out, ",\"at\":%ld", p); end_rec(); LeaveCriticalSection(&lock);
    return ((dsb_l_t)vt_orig(b, 13))(b, p);
}
typedef HRESULT(WINAPI *ds_csb_t)(void *, const void *, void **, void *);
typedef struct { DWORD size, flags, bytes, res; WAVEFORMATEX *fmt; } dsbdesc_t; /* DSBUFFERDESC */
static HRESULT WINAPI ds_create_buffer(void *ds, const void *desc, void **b, void *u);
static void hook_ds(void *ds) {
    static const int idx[] = {3};
    static void *const to[] = {(void *)ds_create_buffer};
    vt_hook(ds, idx, to, 1);
}
static HRESULT WINAPI ds_create_buffer(void *ds, const void *desc, void **b, void *u) {
    HRESULT r = ((ds_csb_t)vt_orig(ds, 3))(ds, desc, b, u);
    if (r < 0 || !b || !*b) return r;
    static const int idx[] = {12, 13, 15, 18};
    static void *const to[] = {(void *)dsb_play, (void *)dsb_pos, (void *)dsb_vol, (void *)dsb_stop};
    vt_hook(*b, idx, to, 4);
    const dsbdesc_t *d = desc;
    EnterCriticalSection(&lock);
    int i = n_bufs < 4096 ? n_bufs++ : 4095; /* shortcut: past 4096 buffers the last id is reused */
    bufs[i].buf = *b, bufs[i].vol = 1;
    REC("dsbuf");
    fprintf(out, ",\"id\":%d,\"bytes\":%lu", i, d->bytes);
    if (d->flags & 1) fputs(",\"primary\":1", out); /* DSBCAPS_PRIMARYBUFFER */
    if (d->fmt) fprintf(out, ",\"hz\":%lu,\"ch\":%u,\"bits\":%u", d->fmt->nSamplesPerSec, d->fmt->nChannels, d->fmt->wBitsPerSample);
    fputs(",\"src\":", out);
    put_a(last_read);
    put_stack();
    end_rec();
    LeaveCriticalSection(&lock);
    return r;
}
H(HRESULT, DirectSoundCreate, (const GUID *g, void **ds, void *u)) {
    HRESULT r = o_DirectSoundCreate(g, ds, u);
    if (r >= 0 && ds && *ds) hook_ds(*ds);
    return r;
}
H(HRESULT, DirectSoundCreate8, (const GUID *g, void **ds, void *u)) {
    HRESULT r = o_DirectSoundCreate8(g, ds, u);
    if (r >= 0 && ds && *ds) hook_ds(*ds);
    return r;
}
static const GUID clsid_ds = {0x47d4d946, 0x62e8, 0x11cf, {0x93, 0xbc, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00}},
                  clsid_ds8 = {0x3901cc3f, 0x84b5, 0x4fa4, {0xba, 0x35, 0xaa, 0x81, 0x72, 0xb8, 0xa0, 0x9b}};
H(HRESULT, CoCreateInstance, (REFCLSID c, LPUNKNOWN o, DWORD x, REFIID i, LPVOID *p)) {
    int top = enter();
    HRESULT r = o_CoCreateInstance(c, o, x, i, p);
    if (top) {
        EnterCriticalSection(&lock);
        REC("com");
        fprintf(out, ",\"clsid\":\"%08lx-%04x-%04x-%02x%02x-%02x%02x%02x%02x%02x%02x\",\"hr\":%ld", c->Data1, c->Data2, c->Data3,
                c->Data4[0], c->Data4[1], c->Data4[2], c->Data4[3], c->Data4[4], c->Data4[5], c->Data4[6], c->Data4[7], (long)r);
        end_rec();
        LeaveCriticalSection(&lock);
        if (r >= 0 && p && *p && (IsEqualGUID(c, &clsid_ds) || IsEqualGUID(c, &clsid_ds8))) hook_ds(*p);
    }
    leave(top);
    return r;
}

/* ---- the hook table ---- */
static FARPROC (WINAPI *o_GetProcAddress)(HMODULE, LPCSTR);
static FARPROC WINAPI h_GetProcAddress(HMODULE m, LPCSTR n);
#define E(dll, fn) {dll, #fn, (void *)h_##fn, (void **)&o_##fn, 0}
static struct hook { const char *dll, *fn; void *to; void **orig; void *real; } hooks[] = {
    E("kernel32", CreateFileA), E("kernel32", CreateFileW), E("kernel32", ReadFile), E("kernel32", WriteFile),
    E("kernel32", FindFirstFileA), E("kernel32", FindFirstFileW), E("kernel32", GetFileAttributesA),
    E("kernel32", GetFileAttributesW), E("kernel32", Sleep), E("kernel32", SleepEx), E("kernel32", GetTickCount),
    E("kernel32", QueryPerformanceCounter), E("kernel32", GetProcAddress),
    E("winmm", timeGetTime), E("winmm", timeSetEvent), E("winmm", waveOutOpen), E("winmm", waveOutWrite),
    E("winmm", waveOutReset), E("winmm", mciSendStringA), E("winmm", mciSendStringW),
    E("user32", SetTimer), E("user32", DispatchMessageA), E("user32", DispatchMessageW), E("user32", CreateWindowExA),
    E("user32", CreateWindowExW), E("user32", MessageBoxA), E("user32", MessageBoxW),
    E("gdi32", SwapBuffers), E("gdi32", BitBlt), E("gdi32", StretchBlt), E("gdi32", StretchDIBits),
    E("gdi32", SetDIBitsToDevice), E("opengl32", wglSwapBuffers),
    E("dsound", DirectSoundCreate), E("dsound", DirectSoundCreate8), E("ole32", CoCreateInstance),
};
#define N_HOOKS (int)(sizeof hooks / sizeof *hooks)

static void resolve(void) {
    for (int i = 0; i < N_HOOKS; i++) {
        if (hooks[i].real) continue;
        HMODULE m = GetModuleHandleA(hooks[i].dll);
        if (!m && strcmp(hooks[i].dll, "opengl32") && strcmp(hooks[i].dll, "dsound")) m = LoadLibraryA(hooks[i].dll);
        if (m && (hooks[i].real = (void *)GetProcAddress(m, hooks[i].fn))) *hooks[i].orig = hooks[i].real;
    }
}

static FARPROC WINAPI h_GetProcAddress(HMODULE m, LPCSTR n) {
    FARPROC r = o_GetProcAddress(m, n);
    resolve(); /* a DLL loaded since (opengl32, dsound) */
    for (int i = 0; r && i < N_HOOKS; i++)
        if ((void *)r == hooks[i].real) return (FARPROC)hooks[i].to;
    return r;
}

/* point one module's imports at the hooks */
static void patch_module(unsigned char *b) {
    IMAGE_NT_HEADERS *nt = (IMAGE_NT_HEADERS *)(b + ((IMAGE_DOS_HEADER *)b)->e_lfanew);
    IMAGE_DATA_DIRECTORY *dd = &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT];
    if (!dd->VirtualAddress) return;
    for (IMAGE_IMPORT_DESCRIPTOR *d = (IMAGE_IMPORT_DESCRIPTOR *)(b + dd->VirtualAddress); d->Name; d++) {
        IMAGE_THUNK_DATA *a = (IMAGE_THUNK_DATA *)(b + d->FirstThunk);
        IMAGE_THUNK_DATA *nm = d->OriginalFirstThunk ? (IMAGE_THUNK_DATA *)(b + d->OriginalFirstThunk) : NULL;
        for (; a->u1.Function; a++, nm = nm ? nm + 1 : NULL) {
            const char *name = nm && !IMAGE_SNAP_BY_ORDINAL(nm->u1.Ordinal) ? (const char *)((IMAGE_IMPORT_BY_NAME *)(b + nm->u1.AddressOfData))->Name : NULL;
            for (int i = 0; i < N_HOOKS; i++) {
                if (!hooks[i].real || (void *)a->u1.Function == hooks[i].to) continue;
                if ((void *)a->u1.Function != hooks[i].real && !(name && !strcmp(name, hooks[i].fn) &&
                                                                  !_strnicmp((char *)b + d->Name, hooks[i].dll, strlen(hooks[i].dll))))
                    continue;
                DWORD old;
                VirtualProtect(&a->u1.Function, sizeof(void *), PAGE_READWRITE, &old);
                a->u1.Function = (ULONG_PTR)hooks[i].to;
                VirtualProtect(&a->u1.Function, sizeof(void *), old, &old);
            }
        }
    }
}

static void *patched[512];
static int n_patched;
static void scan(void) {
    HANDLE s = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, 0);
    MODULEENTRY32 me = {sizeof me};
    if (s == INVALID_HANDLE_VALUE) return;
    resolve();
    for (BOOL ok = Module32First(s, &me); ok; ok = Module32Next(s, &me)) {
        int k;
        for (k = 0; k < n_patched && patched[k] != me.modBaseAddr; k++) {}
        if (k < n_patched || n_patched == 512) continue;
        patched[n_patched++] = me.modBaseAddr;
        const char *m = me.szModule;
        if (!_strnicmp(me.szExePath, self_dir, strlen(self_dir)) || !_stricmp(m, "ntdll.dll") || !_stricmp(m, "kernel32.dll") ||
            !_stricmp(m, "kernelbase.dll") || !_stricmp(m, "win32u.dll"))
            continue;
        if (!_strnicmp(me.szExePath, game_dir, strlen(game_dir)) && n_game_mods < 64) {
            game_mods[n_game_mods].lo = (char *)me.modBaseAddr;
            game_mods[n_game_mods++].hi = (char *)me.modBaseAddr + me.modBaseSize;
        }
        patch_module(me.modBaseAddr);
    }
    CloseHandle(s);
}

/* ---- breakpoints from FC_HOOKS ---- */
#define N_BP 256
static struct { char *a; char name[64]; int mode; unsigned char byte; volatile LONG hits, sec; } bp[N_BP];
static int n_bp;
static DWORD step_tls; /* the breakpoint (index + 1) a thread is single-stepping past */
enum { ONCE, COUNT, EVERY };

static void poke(char *a, unsigned char v) {
    DWORD old;
    VirtualProtect(a, 1, PAGE_EXECUTE_READWRITE, &old);
    *(volatile unsigned char *)a = v;
    VirtualProtect(a, 1, old, &old);
    FlushInstructionCache(GetCurrentProcess(), a, 1);
}

/* a cp932 string at p? (readable, 3+ printable bytes, NUL-ended within 128) */
static int str_at(ULONG_PTR p, char *buf) {
    MEMORY_BASIC_INFORMATION mi;
    if (p < 0x10000 || !VirtualQuery((void *)p, &mi, sizeof mi) || mi.State != MEM_COMMIT ||
        (mi.Protect & (PAGE_GUARD | PAGE_NOACCESS)) || p + 128 > (ULONG_PTR)mi.BaseAddress + mi.RegionSize)
        return 0;
    const unsigned char *s = (const unsigned char *)p;
    int n = 0;
    while (n < 127 && s[n] && (s[n] >= 0x20 || s[n] == '\t' || s[n] == '\n' || s[n] == '\r')) n++;
    if (n < 3 || (n == 127 && s[n])) return 0;
    memcpy(buf, s, n);
    buf[n] = 0;
    return n;
}

static void bp_record(int i, CONTEXT *c) {
#ifdef _WIN64
    ULONG_PTR r[] = {c->Rcx, c->Rdx, c->R8, c->R9, c->Rax, c->Rbx, c->Rsi, c->Rdi}, *sp = (ULONG_PTR *)c->Rsp + 5; /* past the return and shadow space */
#else
    ULONG_PTR r[] = {c->Eax, c->Ecx, c->Edx, c->Ebx, c->Esi, c->Edi, c->Ebp, c->Esp}, *sp = (ULONG_PTR *)c->Esp + 1;
#endif
    char buf[128];
    EnterCriticalSection(&lock);
    REC("bp");
    fputs(",\"name\":", out);
    put_a(bp[i].name);
    fputs(",\"r\":[", out);
    for (int k = 0; k < 8; k++) fprintf(out, "%s%llu", k ? "," : "", (unsigned long long)r[k]);
    fputs("],\"s\":[", out);
    for (int k = 0; k < 4; k++) fprintf(out, "%s%llu", k ? "," : "", (unsigned long long)sp[k]);
    fputs("],\"str\":{", out);
    int first = 1;
    for (int k = 0; k < 12; k++)
        if (str_at(k < 8 ? r[k] : sp[k - 8], buf)) {
            fprintf(out, "%s\"%s%d\":", first ? "" : ",", k < 8 ? "r" : "s", k < 8 ? k : k - 8);
            put_a(buf);
            first = 0;
        }
    fputs("}", out);
    end_rec();
    LeaveCriticalSection(&lock);
}

static LONG CALLBACK fc_veh(EXCEPTION_POINTERS *e) {
    DWORD code = e->ExceptionRecord->ExceptionCode;
    if (code == EXCEPTION_SINGLE_STEP) {
        int i = (int)(ULONG_PTR)TlsGetValue(step_tls) - 1;
        if (i < 0) return EXCEPTION_CONTINUE_SEARCH;
        TlsSetValue(step_tls, NULL);
        poke(bp[i].a, 0xcc); /* armed again */
        return EXCEPTION_CONTINUE_EXECUTION;
    }
    if (code != EXCEPTION_BREAKPOINT) return EXCEPTION_CONTINUE_SEARCH;
    char *a = (char *)e->ExceptionRecord->ExceptionAddress;
    int i;
    for (i = 0; i < n_bp && bp[i].a != a; i++) {}
    if (i == n_bp) return EXCEPTION_CONTINUE_SEARCH;
    LONG n = InterlockedIncrement(&bp[i].hits);
    InterlockedIncrement(&bp[i].sec);
    if (bp[i].mode == ONCE ? n == 1 : bp[i].mode == EVERY && n <= 200) bp_record(i, e->ContextRecord);
    PC(e->ContextRecord) = (ULONG_PTR)a;
    if (*(volatile unsigned char *)a == 0xcc) poke(a, bp[i].byte); /* other threads miss it until it is armed again */
    if (bp[i].mode != ONCE) {
        TlsSetValue(step_tls, (void *)(ULONG_PTR)(i + 1));
        e->ContextRecord->EFlags |= 0x100; /* trap after this instruction */
    }
    return EXCEPTION_CONTINUE_EXECUTION;
}

static void bp_install(const char *path) {
    FILE *f = fopen(path, "r");
    char line[512];
    if (!f) return;
    unsigned char *b = (unsigned char *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS *nt = (IMAGE_NT_HEADERS *)(b + ((IMAGE_DOS_HEADER *)b)->e_lfanew);
    ULONG_PTR slide = (ULONG_PTR)b - nt->OptionalHeader.ImageBase;
    while (n_bp < N_BP && fgets(line, sizeof line, f)) {
        char *t1 = strchr(line, '\t'), *t2;
        if (line[0] == '#' || !t1) continue;
        *t1++ = 0;
        if ((t2 = strpbrk(t1, "\t\r\n"))) *t2++ = 0;
        bp[n_bp].a = (char *)(ULONG_PTR)(strtoull(line, NULL, 16) + slide);
        lstrcpynA(bp[n_bp].name, t1, sizeof bp[n_bp].name);
        bp[n_bp].mode = t2 && !strncmp(t2, "count", 5) ? COUNT : t2 && !strncmp(t2, "every", 5) ? EVERY : ONCE;
        MEMORY_BASIC_INFORMATION mi;
        if (!VirtualQuery(bp[n_bp].a, &mi, sizeof mi) || mi.State != MEM_COMMIT || !(mi.Protect & 0xf0)) continue; /* not code */
        n_bp++;
    }
    fclose(f);
    AddVectoredExceptionHandler(1, fc_veh);
    for (int i = 0; i < n_bp; i++) {
        bp[i].byte = *(unsigned char *)bp[i].a;
        poke(bp[i].a, 0xcc);
    }
}

/* once a second: the counts, and modules loaded since */
static void counts(void) {
    LONG v[] = {InterlockedExchange(&c_present, 0), InterlockedExchange(&c_sleep, 0), InterlockedExchange(&c_sleep_ms, 0),
                InterlockedExchange(&sleep_min, -1), InterlockedExchange(&sleep_max, 0), InterlockedExchange(&c_tgt, 0),
                InterlockedExchange(&c_qpc, 0), InterlockedExchange(&c_gtc, 0), InterlockedExchange(&c_wout, 0),
                InterlockedExchange(&c_probe, 0)};
    static const char *k[] = {"present", "sleep", "sleep_ms", "sleep_min", "sleep_max", "tgt", "qpc", "gtc", "wout", "probe"};
    EnterCriticalSection(&lock);
    REC("sec");
    for (int i = 0; i < 10; i++)
        if (v[i] > 0 || (i == 3 && v[1])) fprintf(out, ",\"%s\":%ld", k[i], v[i]);
    int first = 1;
    for (int i = 0; i < n_bp; i++) {
        LONG h = InterlockedExchange(&bp[i].sec, 0);
        if (!h || bp[i].mode == ONCE) continue;
        fputs(first ? ",\"hits\":{" : ",", out);
        put_a(bp[i].name);
        fprintf(out, ":%ld", h);
        first = 0;
    }
    if (!first) fputc('}', out);
    end_rec();
    LeaveCriticalSection(&lock);
}
static DWORD WINAPI ticker(void *u) {
    (void)u;
    TlsSetValue(busy_tls, (void *)1); /* nothing this thread does is the game's */
    for (;;) {
        Sleep(1000);
        scan();
        counts();
    }
    return 0;
}

static void install(void) {
    const char *p = getenv("FC_OUT");
    out = fopen(p ? p : "C:\\fctrace.jsonl", "w");
    if (!out) return;
    QueryPerformanceFrequency(&qf);
    QueryPerformanceCounter(&q0);
    InitializeCriticalSection(&lock);
    busy_tls = TlsAlloc();
    step_tls = TlsAlloc();
    if ((p = getenv("FC_SHOT_EVERY"))) shot_every = atol(p);
    if ((p = getenv("FC_SHOT_REQ"))) lstrcpynA(shot_req, p, sizeof shot_req);
    char exe[MAX_PATH], *s;
    GetModuleFileNameA(NULL, exe, sizeof exe);
    lstrcpynA(game_dir, exe, sizeof game_dir);
    if ((s = strrchr(game_dir, '\\'))) s[1] = 0;
    GetModuleFileNameA(self, self_dir, sizeof self_dir);
    if ((s = strrchr(self_dir, '\\'))) s[1] = 0;
    exe_lo = (char *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS *nt = (IMAGE_NT_HEADERS *)(exe_lo + ((IMAGE_DOS_HEADER *)exe_lo)->e_lfanew);
    exe_hi = exe_lo + nt->OptionalHeader.SizeOfImage;
    exe_slide = (ULONG_PTR)exe_lo - nt->OptionalHeader.ImageBase;
    if ((p = getenv("FC_HOOKS"))) bp_install(p);
    fprintf(out, "{\"t\":\"meta\",\"schema\":1,\"engine\":\"fctrace\",\"bits\":%d,\"exe\":", BITS);
    put_a(exe);
    fputs(",\"bp\":[", out);
    for (int i = 0; i < n_bp; i++) {
        fprintf(out, "%s{\"addr\":\"%p\",\"name\":", i ? "," : "", bp[i].a);
        put_a(bp[i].name);
        fprintf(out, ",\"mode\":%d}", bp[i].mode);
    }
    fputs("]}\n", out);
    fflush(out);
    scan();
    CloseHandle(CreateThread(NULL, 0, ticker, NULL, 0, NULL));
}

BOOL WINAPI DllMain(HINSTANCE h, DWORD why, LPVOID r) {
    (void)r;
    if (why == DLL_PROCESS_ATTACH) self = h, install();
    return TRUE;
}
