 .syntax unified
.cpu arm7tdmi
.arm
@ Original MIT diagnostic. Each ten-row band identifies one calculation case.
@ Green means the guest verified both the result and selected architectural flags.
.section .text.entry, "ax", %progbits
.global _start
_start:
    ldr r0, =0x04000000
    ldr r1, =0x0403
    strh r1, [r0]
    ldr r8, =0x06000000
    ldr r9, =0x03000000
    mov r10, #0
    strh r10, [r9, #4]
    mov r11, #0

@ Case 1: adds r2, r0, r1 with independent expected result.
    mov r11, #1
    ldr r0, =0xffffffff
    ldr r1, =0x00000001
    adds r2, r0, r1
    bne fail_1
    bcc fail_1
    bvs fail_1
    ldr r3, =0x00000000
    cmp r2, r3
    bne fail_1
    ldr r5, =0x03e0
    b draw_1
fail_1:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_1:
    ldr r6, =2400
paint_1:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_1
    b next_1
    .ltorg
next_1:

@ Case 2: adds r2, r0, r1 with independent expected result.
    mov r11, #2
    ldr r0, =0x7fffffff
    ldr r1, =0x00000001
    adds r2, r0, r1
    bvc fail_2
    bcs fail_2
    ldr r3, =0x80000000
    cmp r2, r3
    bne fail_2
    ldr r5, =0x03e0
    b draw_2
fail_2:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_2:
    ldr r6, =2400
paint_2:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_2
    b next_2
    .ltorg
next_2:

@ Case 3: subs r2, r0, r1 with independent expected result.
    mov r11, #3
    ldr r0, =0x00000000
    ldr r1, =0x00000001
    subs r2, r0, r1
    bcs fail_3
    bpl fail_3
    ldr r3, =0xffffffff
    cmp r2, r3
    bne fail_3
    ldr r5, =0x03e0
    b draw_3
fail_3:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_3:
    ldr r6, =2400
paint_3:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_3
    b next_3
    .ltorg
next_3:

@ Case 4: rsbs r2, r0, r1 with independent expected result.
    mov r11, #4
    ldr r0, =0x00000003
    ldr r1, =0x0000000a
    rsbs r2, r0, r1
    bcc fail_4
    ldr r3, =0x00000007
    cmp r2, r3
    bne fail_4
    ldr r5, =0x03e0
    b draw_4
fail_4:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_4:
    ldr r6, =2400
paint_4:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_4
    b next_4
    .ltorg
next_4:

@ Case 5: adcs r2, r0, r1 with independent expected result.
    mov r11, #5
    ldr r0, =0xffffffff
    ldr r1, =0x00000000
    cmp r0, r0
    adcs r2, r0, r1
    bcc fail_5
    bne fail_5
    ldr r3, =0x00000000
    cmp r2, r3
    bne fail_5
    ldr r5, =0x03e0
    b draw_5
fail_5:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_5:
    ldr r6, =2400
paint_5:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_5
    b next_5
    .ltorg
next_5:

@ Case 6: sbcs r2, r0, r1 with independent expected result.
    mov r11, #6
    ldr r0, =0x00000000
    ldr r1, =0x00000001
    cmp r0, r1
    sbcs r2, r0, r1
    bcs fail_6
    ldr r3, =0xfffffffe
    cmp r2, r3
    bne fail_6
    ldr r5, =0x03e0
    b draw_6
fail_6:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_6:
    ldr r6, =2400
paint_6:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_6
    b next_6
    .ltorg
next_6:

@ Case 7: rscs r2, r0, r1 with independent expected result.
    mov r11, #7
    ldr r0, =0x00000003
    ldr r1, =0x0000000a
    cmp r0, r0
    rscs r2, r0, r1
    bcc fail_7
    ldr r3, =0x00000007
    cmp r2, r3
    bne fail_7
    ldr r5, =0x03e0
    b draw_7
fail_7:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_7:
    ldr r6, =2400
paint_7:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_7
    b next_7
    .ltorg
next_7:

@ Case 8: eors r2, r0, r1 with independent expected result.
    mov r11, #8
    ldr r0, =0x55aa55aa
    ldr r1, =0xffff0000
    eors r2, r0, r1
    
    ldr r3, =0xaa5555aa
    cmp r2, r3
    bne fail_8
    ldr r5, =0x03e0
    b draw_8
fail_8:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_8:
    ldr r6, =2400
paint_8:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_8
    b next_8
    .ltorg
next_8:

@ Case 9: bics r2, r0, r1 with independent expected result.
    mov r11, #9
    ldr r0, =0xffffffff
    ldr r1, =0xff00ff00
    bics r2, r0, r1
    
    ldr r3, =0x00ff00ff
    cmp r2, r3
    bne fail_9
    ldr r5, =0x03e0
    b draw_9
fail_9:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_9:
    ldr r6, =2400
paint_9:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_9
    b next_9
    .ltorg
next_9:

@ Case 10: mvns r2, r0 with independent expected result.
    mov r11, #10
    ldr r0, =0x12345678
    ldr r1, =0x00000000
    mvns r2, r0
    
    ldr r3, =0xedcba987
    cmp r2, r3
    bne fail_10
    ldr r5, =0x03e0
    b draw_10
fail_10:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_10:
    ldr r6, =2400
paint_10:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_10
    b next_10
    .ltorg
next_10:

@ Case 11: movs r2, r0, lsl #1 with independent expected result.
    mov r11, #11
    ldr r0, =0x80000001
    ldr r1, =0x00000000
    movs r2, r0, lsl #1
    bcc fail_11
    ldr r3, =0x00000002
    cmp r2, r3
    bne fail_11
    ldr r5, =0x03e0
    b draw_11
fail_11:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_11:
    ldr r6, =2400
paint_11:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_11
    b next_11
    .ltorg
next_11:

@ Case 12: movs r2, r0, lsr #32 with independent expected result.
    mov r11, #12
    ldr r0, =0x80000001
    ldr r1, =0x00000000
    movs r2, r0, lsr #32
    bcc fail_12
    bne fail_12
    ldr r3, =0x00000000
    cmp r2, r3
    bne fail_12
    ldr r5, =0x03e0
    b draw_12
fail_12:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_12:
    ldr r6, =2400
paint_12:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_12
    b next_12
    .ltorg
next_12:

@ Case 13: movs r2, r0, asr #32 with independent expected result.
    mov r11, #13
    ldr r0, =0x80000001
    ldr r1, =0x00000000
    movs r2, r0, asr #32
    bcc fail_13
    bpl fail_13
    ldr r3, =0xffffffff
    cmp r2, r3
    bne fail_13
    ldr r5, =0x03e0
    b draw_13
fail_13:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_13:
    ldr r6, =2400
paint_13:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_13
    b next_13
    .ltorg
next_13:

@ Case 14: movs r2, r0, rrx with independent expected result.
    mov r11, #14
    ldr r0, =0x00000003
    ldr r1, =0x00000000
    cmp r0, r0
    movs r2, r0, rrx
    bcc fail_14
    ldr r3, =0x80000001
    cmp r2, r3
    bne fail_14
    ldr r5, =0x03e0
    b draw_14
fail_14:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_14:
    ldr r6, =2400
paint_14:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_14
    b next_14
    .ltorg
next_14:

@ Case 15: muls r2, r0, r1 with independent expected result.
    mov r11, #15
    ldr r0, =0xffffffff
    ldr r1, =0x00000003
    muls r2, r0, r1
    bpl fail_15
    ldr r3, =0xfffffffd
    cmp r2, r3
    bne fail_15
    ldr r5, =0x03e0
    b draw_15
fail_15:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_15:
    ldr r6, =2400
paint_15:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_15
    b next_15
    .ltorg
next_15:

@ Case 16: bne fail with independent expected result.
    mov r11, #16
    ldr r0, =0xffffffff
    ldr r1, =0x00000003
    mov r3, #7
    mla r2, r0, r1, r3
    cmn r0, #1
    bne fail_16
    bcc fail_16
    teq r1, #3
    bne fail_16
    
    ldr r3, =0x00000004
    cmp r2, r3
    bne fail_16
    ldr r5, =0x03e0
    b draw_16
fail_16:
    cmp r10, #0
    moveq r10, r11
    strh r10, [r9, #4]
    mov r5, #0x1f
draw_16:
    ldr r6, =2400
paint_16:
    strh r5, [r8], #2
    subs r6, r6, #1
    bne paint_16
    b next_16
    .ltorg
next_16:

@ Publish completion only after every case has drawn; preserve the first failure.
    mov r0, #0x5d
    strh r0, [r9]
    cmp r10, #0
    moveq r0, #1
    movne r0, #0
    strh r0, [r9, #2]
    b calculations_terminal
    .ltorg
    .org 0x1000
.global calculations_terminal
calculations_terminal:
    b calculations_terminal
    .word 0, 0
