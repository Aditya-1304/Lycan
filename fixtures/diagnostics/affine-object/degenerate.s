@ Standalone ARM translation of mGBA suite degenerateObjTransform.
@ Copyright (c) 2015 Jeffrey Pfau; MIT license in mgba-LICENSE.
@ The original palette, texels, six objects and singular matrices are preserved.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
_start:
    ldr r0, =0x04000000
    mov r1, #0x80
    strh r1, [r0]
    ldr r4, =0x07000000
    mov r1, #0x200
    mov r2, #128
disable:
    strh r1, [r4], #8
    subs r2, r2, #1
    bne disable
    @ The suite uses a white backdrop for source pixels outside the OBJ image.
    ldr r4, =0x05000000
    ldr r1, =0x7fff
    strh r1, [r4]
    ldr r4, =0x05000220
    ldr r3, =colors
    mov r2, #9
palette:
    ldrh r1, [r3], #2
    strh r1, [r4], #2
    subs r2, r2, #1
    bne palette
    ldr r4, =0x06014800
    mov r2, #64
tile:
    ldr r3, =texels
    mov r5, #8
word:
    ldr r1, [r3], #4
    str r1, [r4], #4
    subs r5, r5, #1
    bne word
    subs r2, r2, #1
    bne tile
    ldr r4, =0x07000008
    ldr r3, =objects
    mov r2, #6
object:
    ldrh r1, [r3], #2
    strh r1, [r4]
    ldrh r1, [r3], #2
    strh r1, [r4, #2]
    ldrh r1, [r3], #2
    strh r1, [r4, #4]
    add r4, r4, #8
    subs r2, r2, #1
    bne object
    ldr r4, =0x07000006
    ldr r3, =matrices
    mov r2, #24
matrix:
    ldrh r1, [r3], #2
    strh r1, [r4], #8
    subs r2, r2, #1
    bne matrix
    ldr r1, =0x1040
    strh r1, [r0]
    ldr r4, =0x03000000
    mov r1, #0xa3
    strh r1, [r4]
    mov r1, #1
    strh r1, [r4, #2]
terminal:
    b terminal
    .ltorg
colors:
    .hword 0x7f1c,0,0x1084,0x2108,0x318c,0x4210,0x5294,0x6318,0x739c
    .balign 4
texels:
    .word 0x12345678,0x45678123,0x78123456,0x23456781
    .word 0x56781234,0x81234567,0x34567812,0x67812345
objects:
    .hword 0x100,0xc000,0x1240, 0x100,0xc240,0x1240
    .hword 0x140,0xc400,0x1240, 0x140,0xc640,0x1240
    .hword 0x100,0xc880,0x1240, 0x140,0xca80,0x1240
matrices:
    .hword 256,0,0,0, 0,256,0,0, 0,0,256,0
    .hword 0,0,0,256, 0,0,0,0, 256,256,256,256
    .space 8
