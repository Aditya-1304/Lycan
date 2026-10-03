 .syntax unified
.cpu arm7tdmi
.arm
@ Original MIT fixture. Copy indexed bitmap data through mirrored work RAM,
@ then redraw from the canonical address using a stack-preserving subroutine.
.section .text.entry, "ax", %progbits
.global _start
_start:
    ldr sp, =0x03010000
    ldr r0, =0x04000000
    ldr r1, =0x0404
    strh r1, [r0]
    ldr r0, =0x05000000
    mov r1, #0x1f
    strh r1, [r0, #2]
    mov r1, #0xe0
    strb r1, [r0, #4]
    ldr r0, =bitmap
    ldr r1, =0x02040000
    ldr r2, =2400
copy_words:
    ldmia r0!, {r4-r7}
    stmia r1!, {r4-r7}
    subs r2, r2, #1
    bne copy_words
@ Unaligned STR aligns down; unaligned LDR rotates the aligned word.
    ldr r0, =0x0204a000
    ldr r1, =0x11223344
    str r1, [r0, #1]
    ldr r2, [r0, #1]
    ldr r3, =0x44112233
    cmp r2, r3
    bne failed
    ldr r0, =0x03008020
    mov r1, #0x5e
    strb r1, [r0]
    ldrb r2, [r0]
    cmp r1, r2
    bne failed
@ Bitmap OBJ VRAM rejects byte writes; BG VRAM duplicates the byte.
    ldr r0, =0x06014000
    mov r1, #0x55
    strb r1, [r0]
    ldr r0, =0x06010000
    mov r1, #2
    strb r1, [r0]
    bl redraw
    ldr r0, =0x03010000
    cmp sp, r0
    bne failed
@ Completion is reachable only after LDM restores the caller's PC and stack.
    ldr r0, =0x03000000
    mov r1, #0x5e
    strh r1, [r0]
    mov r1, #1
    strh r1, [r0, #2]
    b copy_terminal
failed:
    b failed
redraw:
    stmdb sp!, {r4-r7, lr}
    ldr r0, =0x02000000
    ldr r1, =0x06000000
    ldr r2, =19200
redraw_loop:
    ldrb r3, [r0], #2
    strb r3, [r1], #2
    subs r2, r2, #1
    bne redraw_loop
    ldmia sp!, {r4-r7, pc}
    .ltorg
.balign 4
bitmap:
    .rept 4800
    .word 0x01010101
    .endr
    .rept 4800
    .word 0x02020202
    .endr
    .org 0xa000
copy_terminal:
    b copy_terminal
    .word 0, 0
