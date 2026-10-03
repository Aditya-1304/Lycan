.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry, "ax", %progbits
.global _start
_start:
    b entry
    .space 188
entry:
    ldr r0, =0x04000000
    ldr r1, =0x0403
    strh r1, [r0]
    ldr r10, =0x0e000000
    ldr r11, =0x03000000
    ldrb r0, [r10]
    cmp r0, #0xa5
    ldrbeq r9, [r10, #1]
    movne r9, #0
    cmp r9, #16
    movhi r9, #0
    mov r0, #0xa5
    strb r0, [r10]
    strb r9, [r10, #1]
    mov r8, #0
    mov r0, #0x73
    strh r0, [r11]
    strh r9, [r11, #2]
    bl draw
frame:
    ldr r0, =0x04000006
leave_vblank:
    ldrh r1, [r0]
    cmp r1, #160
    bhs leave_vblank
wait_vblank:
    ldrh r1, [r0]
    cmp r1, #160
    blo wait_vblank
    ldr r0, =0x04000130
    ldrh r1, [r0]
    and r1, r1, #1
    cmp r1, #0
    bne released
    cmp r8, #0
    bne record
    cmp r9, #16
    addlo r9, r9, #1
    strb r9, [r10, #1]
    mov r8, #1
    b record
released:
    mov r8, #0
record:
    strh r9, [r11, #2]
    bl draw
    b frame

@ The guest owns score and rendering. Each point fills one eight-pixel cell;
@ cleared cells are black, so a restored score has an exact complete-frame image.
draw:
    ldr r2, =0x06000000
    mov r3, #8
    mov r6, r9, lsl #3
row:
    mov r4, #0
pixel:
    cmp r4, r6
    movlo r5, #0x03e0
    movhs r5, #0
    strh r5, [r2], #2
    add r4, r4, #1
    cmp r4, #128
    blo pixel
    add r2, r2, #224
    subs r3, r3, #1
    bne row
    bx lr
    .ltorg
    .asciz "SRAM_V113"
    .balign 4
