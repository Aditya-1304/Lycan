.syntax unified
.cpu arm7tdmi
.arm

@ Original controlled-startup demo. No BIOS services or retail header are required.
@ Eight horizontal bands cover the complete 240x160 mode 3 framebuffer.
.section .text.entry, "ax", %progbits
.global _start
.type _start, %function
_start:
    ldr r0, =0x04000000
    ldr r1, =0x0403
    strh r1, [r0]
    ldr r2, =0x06000000

@ Each band contains twenty rows. Post-indexed stores advance through guest VRAM.
.macro band color
    ldr r3, =\color
    ldr r4, =4800
1:
    strh r3, [r2], #2
    subs r4, r4, #1
    bne 1b
.endm
    band 0x001f
    band 0x03e0
    band 0x7c00
    band 0x03ff
    band 0x7c1f
    band 0x7fe0
    band 0x7fff
    band 0x0000

@ The runner requires both this completion identifier and the success result.
    ldr r4, =0x03000000
    mov r5, #0x5a
    strh r5, [r4]
    mov r5, #1
    strh r5, [r4, #2]
    b pixels_terminal
    .ltorg

@ Branch over padding rather than executing it. The manifest binds this exact PC.
    .org 0x200
.global pixels_terminal
pixels_terminal:
    b pixels_terminal
    @ Padding permits the ARM instruction pipeline to fetch past the terminal branch.
    .word 0, 0
.size _start, . - _start
