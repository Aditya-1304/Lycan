.syntax unified
.cpu arm7tdmi
.arm

@ Original MIT mode-4 fixture. Packed halfword stores initialize distinct index
@ pairs in both bitmap pages; all palette and page changes are guest-driven.
.section .text.entry, "ax", %progbits
.global _start
.type _start, %function
_start:
    ldr r0, =0x04000000
    ldr r1, =0x0404
    strh r1, [r0]
    ldr r2, =0x05000000
    mov r1, #0x1f
    strh r1, [r2]
    ldr r1, =0x03e0
    strh r1, [r2, #2]
    ldr r1, =0x7c00
    strh r1, [r2, #4]
    ldr r1, =0x7fff
    strh r1, [r2, #6]
    ldr r3, =0x06000000
    ldr r4, =0x0600a000
    ldr r5, =19200
    ldr r6, =0x0100
    ldr r7, =0x0302
fill:
    strh r6, [r3], #2
    strh r7, [r4], #2
    subs r5, r5, #1
    bne fill
    ldr r9, =0x03000000
    ldr r10, =0x04000130
    mov r1, #0x5c
    strh r1, [r9]
    mov r1, #0
    strh r1, [r9, #2]
    strh r1, [r9, #4]
frame:
@ Wait for a new VBlank before modifying display state, preserving the earlier
@ demos' master-clock cadence and scanline publication behavior.
wait_visible:
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs wait_visible
wait_vblank:
    ldrh r1, [r0, #4]
    tst r1, #1
    beq wait_vblank
    ldrh r8, [r10]
    ldr r1, =0x0404
    mov r3, #0
    tst r8, #1
    orreq r1, r1, #0x10
    moveq r3, #1
    strh r1, [r0]
    strh r3, [r9, #2]
    ldr r1, =0x7fff
    mov r3, #0
    tst r8, #2
    moveq r1, #0x1f
    moveq r3, #1
    strh r1, [r2, #6]
    strh r3, [r9, #4]
    b frame
    .ltorg
    .word 0, 0
.size _start, . - _start
