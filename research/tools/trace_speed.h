/* trace_speed.h: run a wine-hosted original N times faster for a trace DLL
 * (research/xuse/tracedll/xtrace.c, research/nitroplus/tracedll/ntrace.c).
 *
 * TRACE_SPEED=N (an integer, default 1: nothing hooked) points the exe's
 * imports of its clocks (timeGetTime, GetTickCount, QueryPerformanceCounter)
 * at a clock that runs N times as fast as the real one, and divides by N
 * the time it asks to wait (Sleep, the timeouts of WaitForSingleObject and
 * its kin, timeSetEvent, SetTimer). The game then lives in its own time, N times
 * the real one: a trace DLL writes its records' "ms" as speed_ms() (that
 * game time), so they compare with a 1x run's. What the exe does not time
 * by these (wine's sound mixing, a movie's clock, its own frame cost) stays
 * real: docs/new-game.md, "Trace speeds".
 *
 * Use: speed_install() once in DllMain, before any record is written. A
 * DLL hook of one of these imports found by name, installed after it, wraps
 * it (xtrace's Sleep frame count); one found by the real function's address
 * goes before it and calls the sp_ function itself (ntrace's Django wait).
 */
#ifndef TRACE_SPEED_H
#define TRACE_SPEED_H
#include <windows.h>
#include <mmsystem.h>
#include <stdlib.h>

static LONGLONG sp_n = 1, sp_q0, sp_freq; /* the factor, real QPC at install, its rate */
static DWORD sp_ms0;                      /* timeGetTime at install */

static LONGLONG sp_qpc(void) { /* game time, in QPC ticks since boot */
    LARGE_INTEGER t;
    QueryPerformanceCounter(&t);
    return sp_q0 + (t.QuadPart - sp_q0) * sp_n;
}
/* game ms since install (the trace records' "ms") */
static long speed_ms(void) { return (long)((sp_qpc() - sp_q0) * 1000 / sp_freq); }
static DWORD sp_div(DWORD ms) { /* a wait in game ms -> real ms (INFINITE and 0 kept, else at least 1) */
    return ms == INFINITE || ms == 0 ? ms : ms / sp_n ? ms / sp_n : 1;
}

static DWORD WINAPI sp_timeGetTime(void) { return sp_ms0 + (DWORD)speed_ms(); }
static DWORD WINAPI sp_GetTickCount(void) { return sp_ms0 + (DWORD)speed_ms(); }
static BOOL WINAPI sp_QueryPerformanceCounter(LARGE_INTEGER *t) { return t->QuadPart = sp_qpc(), TRUE; }
static VOID WINAPI sp_Sleep(DWORD ms) { Sleep(sp_div(ms)); }
static DWORD WINAPI sp_SleepEx(DWORD ms, BOOL a) { return SleepEx(sp_div(ms), a); }
static DWORD WINAPI sp_WaitForSingleObject(HANDLE h, DWORD ms) { return WaitForSingleObject(h, sp_div(ms)); }
static DWORD WINAPI sp_WaitForSingleObjectEx(HANDLE h, DWORD ms, BOOL a) { return WaitForSingleObjectEx(h, sp_div(ms), a); }
static DWORD WINAPI sp_WaitForMultipleObjects(DWORD n, const HANDLE *h, BOOL all, DWORD ms) {
    return WaitForMultipleObjects(n, h, all, sp_div(ms));
}
static DWORD WINAPI sp_MsgWaitForMultipleObjects(DWORD n, const HANDLE *h, BOOL all, DWORD ms, DWORD w) {
    return MsgWaitForMultipleObjects(n, h, all, sp_div(ms), w);
}
static DWORD WINAPI sp_MsgWaitForMultipleObjectsEx(DWORD n, const HANDLE *h, DWORD ms, DWORD w, DWORD f) {
    return MsgWaitForMultipleObjectsEx(n, h, sp_div(ms), w, f);
}
static MMRESULT WINAPI sp_timeSetEvent(UINT ms, UINT res, LPTIMECALLBACK cb, DWORD_PTR u, UINT f) {
    return timeSetEvent(sp_div(ms), res, cb, u, f);
}
static UINT_PTR WINAPI sp_SetTimer(HWND w, UINT_PTR id, UINT ms, TIMERPROC p) { return SetTimer(w, id, sp_div(ms), p); }

/* point the exe's import of fn at `to`, found by the real function's address
 * (a rebuilt import table, as an unpacked exe's, may have no names) */
static void sp_hook(const char *dll, const char *fn, void *to) {
    unsigned char *b = (unsigned char *)GetModuleHandleA(NULL);
    IMAGE_NT_HEADERS *nt = (IMAGE_NT_HEADERS *)(b + ((IMAGE_DOS_HEADER *)b)->e_lfanew);
    IMAGE_IMPORT_DESCRIPTOR *d =
        (IMAGE_IMPORT_DESCRIPTOR *)(b + nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress);
    HMODULE m = GetModuleHandleA(dll);
    DWORD real = m ? (DWORD)GetProcAddress(m, fn) : 0;
    for (; real && d->Name; d++)
        for (IMAGE_THUNK_DATA *a = (IMAGE_THUNK_DATA *)(b + d->FirstThunk); a->u1.Function; a++)
            if (a->u1.Function == real) {
                DWORD old;
                VirtualProtect(&a->u1.Function, 4, PAGE_READWRITE, &old);
                a->u1.Function = (DWORD)to;
                VirtualProtect(&a->u1.Function, 4, old, &old);
            }
}

/* returns the factor (1: nothing hooked) */
static long speed_install(void) {
    LARGE_INTEGER f, t;
    const char *p = getenv("TRACE_SPEED");
    QueryPerformanceFrequency(&f);
    QueryPerformanceCounter(&t);
    sp_freq = f.QuadPart, sp_q0 = t.QuadPart, sp_ms0 = timeGetTime();
    if (!p || atol(p) <= 1) return 1;
    sp_n = atol(p);
    LoadLibraryA("winmm.dll");
    static const struct { const char *dll, *fn; void *to; } h[] = {
        {"winmm.dll", "timeGetTime", (void *)sp_timeGetTime},
        {"winmm.dll", "timeSetEvent", (void *)sp_timeSetEvent},
        {"kernel32.dll", "GetTickCount", (void *)sp_GetTickCount},
        {"kernel32.dll", "QueryPerformanceCounter", (void *)sp_QueryPerformanceCounter},
        {"kernel32.dll", "Sleep", (void *)sp_Sleep},
        {"kernel32.dll", "SleepEx", (void *)sp_SleepEx},
        {"kernel32.dll", "WaitForSingleObject", (void *)sp_WaitForSingleObject},
        {"kernel32.dll", "WaitForSingleObjectEx", (void *)sp_WaitForSingleObjectEx},
        {"kernel32.dll", "WaitForMultipleObjects", (void *)sp_WaitForMultipleObjects},
        {"user32.dll", "MsgWaitForMultipleObjects", (void *)sp_MsgWaitForMultipleObjects},
        {"user32.dll", "MsgWaitForMultipleObjectsEx", (void *)sp_MsgWaitForMultipleObjectsEx},
        {"user32.dll", "SetTimer", (void *)sp_SetTimer},
    };
    for (unsigned i = 0; i < sizeof h / sizeof *h; i++) sp_hook(h[i].dll, h[i].fn, h[i].to);
    return (long)sp_n;
}
#endif
