.syntax unified
.cpu arm7tdmi
.arm
@ Original MIT diagnostic. Each case changes WAITCNT, then executes in its
@ selected ROM alias. Markers delimit the measured interval after execution.
.section .text.entry, "ax", %progbits
.global _start
_start:
    b setup_0

.macro timing_case index, setting, window, thumb, expected
.arm
.org 0x100 + \index * 0x100
setup_\index:
    ldr r0, =0x04000204
    ldr r1, =\setting
    strh r1, [r0]
    ldr r7, =0x02000000
    ldr r4, =\window + 0x4000
    ldr r8, =0x080001c0 + \index * 0x100
    ldr r0, =\window + 0x140 + \index * 0x100 + \thumb
    bx r0
    .ltorg
.org 0x140 + \index * 0x100
.if \thumb
.thumb
.endif
begin_\index:
    mov r6, r8
    @ Four work-RAM loads leave the cartridge bus free to fill the opcode FIFO.
    .rept 4
    ldr r5, [r7]
    mov r6, r8
    .endr
    @ Cartridge data reads invalidate the instruction stream and charge N/S beats.
    ldrh r5, [r4]
    ldr r5, [r4]
end_\index:
    mov r6, r8
    bx r8
.arm
.org 0x1c0 + \index * 0x100
report_\index:
    ldr r0, =0x03000008 + \index * 2
    ldr r1, =\expected
    strh r1, [r0]
.if \index < 23
    ldr r0, =0x08000200 + \index * 0x100
.else
    ldr r0, =0x08002000
.endif
    bx r0
    .ltorg
.endm
timing_case 0, 0, 0x8000000, 0, 121
timing_case 1, 0, 0x8000000, 1, 88
timing_case 2, 0, 0xa000000, 0, 155
timing_case 3, 0, 0xa000000, 1, 100
timing_case 4, 0, 0xc000000, 0, 223
timing_case 5, 0, 0xc000000, 1, 124
timing_case 6, 1754, 0x8000000, 0, 88
timing_case 7, 1754, 0x8000000, 1, 66
timing_case 8, 1754, 0xa000000, 0, 88
timing_case 9, 1754, 0xa000000, 1, 66
timing_case 10, 1754, 0xc000000, 0, 88
timing_case 11, 1754, 0xc000000, 1, 66
timing_case 12, 16384, 0x8000000, 0, 85
timing_case 13, 16384, 0x8000000, 1, 64
timing_case 14, 16384, 0xa000000, 0, 127
timing_case 15, 16384, 0xa000000, 1, 72
timing_case 16, 16384, 0xc000000, 0, 211
timing_case 17, 16384, 0xc000000, 1, 112
timing_case 18, 18138, 0x8000000, 0, 60
timing_case 19, 18138, 0x8000000, 1, 54
timing_case 20, 18138, 0xa000000, 0, 60
timing_case 21, 18138, 0xa000000, 1, 54
timing_case 22, 18138, 0xc000000, 0, 60
timing_case 23, 18138, 0xc000000, 1, 54
.arm
.org 0x2000
    ldr r0, =0x03000000
    mov r1, #0x60
    strh r1, [r0]
    mov r1, #1
    strh r1, [r0, #2]
    str r5, [r0, #4]
terminal:
    b terminal
    .ltorg
.org 0x4000
.word 0x12345678
