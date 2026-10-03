.syntax unified
.cpu arm7tdmi
.arm

@ Original MIT controlled-startup fixture. The guest owns position and drawing;
@ the frontend only supplies logical input and presents completed scanlines.
.section .text.entry, "ax", %progbits
.global _start
.type _start, %function
_start:
    ldr r0, =0x04000000
    ldr r1, =0x0403
    strh r1, [r0]
    mov r2, #112                 @ Top-left x; a 16x16 square fits through x=224.
    mov r3, #72                  @ Top-left y; the last valid row begins at y=144.
    ldr r9, =0x060087e0          @ VRAM + (72*240 + 112)*2.
    ldr r11, =0x03000000         @ Mailbox: identifier, x, y, movement-frame count.
    ldr r10, =0x04000130         @ KEYINPUT uses its own halfword base address.
    mov r12, #0
    mov r1, #0x5b
    strh r1, [r11]

@ Unrolled control flow keeps the required instruction subset small. Each macro
@ writes exactly 16 rows and skips the rest of each 240-pixel guest scanline.
.macro square color
    mov r4, #\color
    mov r7, r9
    mov r5, #16
1:
    mov r6, #16
2:
    strh r4, [r7], #2
    subs r6, r6, #1
    bne 2b
    add r7, r7, #448
    subs r5, r5, #1
    bne 1b
.endm
    square 0x1f
    strh r2, [r11, #2]
    strh r3, [r11, #4]
    strh r12, [r11, #6]

frame:
@ First leave the previous VBlank, then wait for the next edge. Waiting only
@ for a set VBlank bit would repeatedly move throughout the same blank period.
wait_visible:
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs wait_visible
wait_vblank:
    ldrh r1, [r0, #4]
    tst r1, #1
    beq wait_vblank

    square 0
    ldrh r8, [r10]
    tst r8, #0x10
    bne no_right
    cmp r2, #224
    addlo r2, r2, #1
    addlo r9, r9, #2
no_right:
    tst r8, #0x20
    bne no_left
    cmp r2, #0
    subhi r2, r2, #1
    subhi r9, r9, #2
no_left:
    tst r8, #0x40
    bne no_up
    cmp r3, #0
    subhi r3, r3, #1
    subhi r9, r9, #480
no_up:
    tst r8, #0x80
    bne no_down
    cmp r3, #144
    addlo r3, r3, #1
    addlo r9, r9, #480
no_down:
    square 0x1f
    add r12, r12, #1
    strh r2, [r11, #2]
    strh r3, [r11, #4]
    strh r12, [r11, #6]
    b frame
    .ltorg
    @ The interpreter fetches two words ahead, including at the final branch.
    .word 0, 0
.size _start, . - _start
