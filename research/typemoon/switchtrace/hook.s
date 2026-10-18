// The trace hook: `script_command`'s loop head (0x177828, `mov x8, xzr`,
// x0 = the next command) branches here. It writes one line through
// svcOutputDebugString, which Ryubing logs:
//   TMT <script index, hex> <offset in the script, hex> <command text>
// then runs the instruction it replaced and goes back. Every register
// and the flags are kept. Addresses are Ghidra's (main at 0x100000);
// mkmain.py links this at the end of the text segment.
    .text
    .globl hook
hook:
    sub sp, sp, #0x120
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x30, [sp, #144]
    mrs x9, nzcv
    str x9, [sp, #160]
    add x2, sp, #176            // the line, 112 bytes
    movz w3, #0x4d54            // "TMT "
    movk w3, #0x2054, lsl #16
    str w3, [x2], #4
    adrp x10, SCRIPT_INDEX_PTR  // GOT slot 0xbe04f0: u16 script index
    ldr x10, [x10, :lo12:SCRIPT_INDEX_PTR]
    ldrh w4, [x10]
    mov w5, #4
    bl hex
    mov w3, #' '
    strb w3, [x2], #1
    adrp x10, SCRIPT_BUF        // 0x1473e04: the loaded script's text
    add x10, x10, :lo12:SCRIPT_BUF
    sub x4, x0, x10
    mov w5, #8
    bl hex
    mov w3, #' '
    strb w3, [x2], #1
    mov w11, #80                // the command, up to its ';'
    mov x12, x0
1:  ldrb w13, [x12], #1
    cmp w13, #';'
    b.eq 2f
    cbz w13, 2f
    strb w13, [x2], #1
    subs w11, w11, #1
    b.ne 1b
2:  add x0, sp, #176
    sub x1, x2, x0
    svc #0x27                   // svcOutputDebugString(x0, x1)
    ldr x9, [sp, #160]
    msr nzcv, x9
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x30, [sp, #144]
    add sp, sp, #0x120
    mov x8, xzr                 // the instruction replaced
    b RETURN                    // 0x17782c

// w5 hex digits of x4 to [x2], x2 advanced; uses x6-x8.
hex:
    sub w5, w5, #1
    lsl w6, w5, #2
    lsr x7, x4, x6
    and x7, x7, #0xf
    add x8, x7, #'0'
    cmp x7, #10
    b.lo 3f
    add x8, x7, #('a' - 10)
3:  strb w8, [x2], #1
    cbnz w5, hex
    ret
