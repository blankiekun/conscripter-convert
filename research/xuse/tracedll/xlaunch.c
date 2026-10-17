/* xlaunch.exe <exe> <dll>: starts <exe> suspended in its own folder, loads
 * <dll> into it with CreateRemoteThread(LoadLibraryA), then resumes it.
 * Waits for the game and returns its exit code. */
#include <windows.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: xlaunch <exe> <dll>\n");
        return 2;
    }
    char dir[MAX_PATH], dll[MAX_PATH], *s;
    GetFullPathNameA(argv[2], MAX_PATH, dll, NULL);
    GetFullPathNameA(argv[1], MAX_PATH, dir, NULL);
    if ((s = strrchr(dir, '\\'))) *s = 0;
    STARTUPINFOA si = {sizeof si};
    PROCESS_INFORMATION pi;
    if (!CreateProcessA(argv[1], NULL, NULL, NULL, FALSE, CREATE_SUSPENDED, NULL, dir, &si, &pi)) {
        fprintf(stderr, "xlaunch: CreateProcess failed (%lu)\n", GetLastError());
        return 3;
    }
    void *mem = VirtualAllocEx(pi.hProcess, NULL, sizeof dll, MEM_COMMIT, PAGE_READWRITE);
    WriteProcessMemory(pi.hProcess, mem, dll, sizeof dll, NULL);
    HANDLE t = CreateRemoteThread(pi.hProcess, NULL, 0,
                                  (LPTHREAD_START_ROUTINE)GetProcAddress(GetModuleHandleA("kernel32"), "LoadLibraryA"),
                                  mem, 0, NULL);
    DWORD mod = 0;
    if (t) {
        WaitForSingleObject(t, 10000);
        GetExitCodeThread(t, &mod);
    }
    if (!mod) {
        fprintf(stderr, "xlaunch: LoadLibrary in the game failed\n");
        TerminateProcess(pi.hProcess, 4);
        return 4;
    }
    fprintf(stderr, "xlaunch: %s loaded at %#lx\n", dll, mod);
    ResumeThread(pi.hThread);
    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD rc = 0;
    GetExitCodeProcess(pi.hProcess, &rc);
    return (int)rc;
}
