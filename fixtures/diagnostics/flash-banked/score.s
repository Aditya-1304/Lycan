.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry, "ax", %progbits
.global _start
_start:
    b entry
    .space 188
entry:
    ldr sp, =0x03007f00
    ldr r0, =0x04000000
    ldr r1, =0x0403
    strh r1, [r0]
    ldr r10, =0x0e000000
    ldr r11, =0x03000000
    @ Verify chip identification before allowing the score scene to run. A
    @ mismatch records a failure mailbox rather than displaying a false success.
    ldr r12, =0x5555
    ldr r6, =0x2aaa
    bl unlock
    mov r0, #0x90
    strb r0, [r10, r12]
    ldrb r0, [r10]
    cmp r0, #0x62
    bne failed
    ldrb r0, [r10, #1]
    cmp r0, #0x13
    bne failed
    mov r0, #0xf0
    strb r0, [r10, r12]
    ldrb r0, [r10]
    cmp r0, #0xa5
    moveq r8, #1
    movne r8, #0
    ldrbeq r9, [r10, #1]
    movne r9, #0
    cmp r9, #16
    movhi r9, #0
    cmp r8, #0
    bleq save_score
    @ Restore the independent bank-one record before drawing bank zero.
    mov r0, #1
    bl select_bank
    ldrb r0, [r10, #1]
    strh r0, [r11, #4]
    mov r0, #0
    bl select_bank
    mov r8, #0
    mov r0, #0x75
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
    bl save_score
    mov r8, #1
    b record
released:
    mov r8, #0
record:
    strh r9, [r11, #2]
    bl draw
    b frame

failed:
    mov r0, #0xff
    strh r0, [r11]
    b failed

@ The guest owns score and rendering. Each point fills one eight-pixel cell;
@ cleared cells are black, so a restored score has an exact complete-frame image.
draw:
    push {r8}
    mov r7, lr
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
    @ Render bank one's independently restored value in a second blue band.
    mov r0, #1
    bl select_bank
    ldrb r0, [r10, #1]
    sub r8, r0, #16
    mov r0, #0
    bl select_bank
    mov r6, r8, lsl #3
    ldr r2, =0x06000f00
    mov r3, #8
bank_row:
    mov r4, #0
bank_pixel:
    cmp r4, r6
    movlo r5, #0x7c00
    movhs r5, #0
    strh r5, [r2], #2
    add r4, r4, #1
    cmp r4, #128
    blo bank_pixel
    add r2, r2, #224
    subs r3, r3, #1
    bne bank_row
    pop {r8}
    bx r7
@ Store the complete score record through the Flash128 command bus. Sector zero
@ belongs to this demo; erasing it permits score bits to transition back to one.
save_score:
    mov r7, lr
    ldr r12, =0x5555
    ldr r6, =0x2aaa
    bl unlock
    mov r0, #0x80
    strb r0, [r10, r12]
    bl unlock
    mov r0, #0x30
    strb r0, [r10]
    bl unlock
    mov r0, #0xa0
    strb r0, [r10, r12]
    mov r0, #0xa5
    strb r0, [r10]
    bl unlock
    mov r0, #0xa0
    strb r0, [r10, r12]
    strb r9, [r10, #1]
    @ Bank one uses a distinct marker and score encoding at the same offsets.
    @ Program it independently, then restore bank zero for ordinary guest reads.
    mov r0, #1
    bl select_bank
    bl unlock
    mov r0, #0x80
    strb r0, [r10, r12]
    bl unlock
    mov r0, #0x30
    strb r0, [r10]
    bl unlock
    mov r0, #0xa0
    strb r0, [r10, r12]
    mov r0, #0x5a
    strb r0, [r10]
    bl unlock
    mov r0, #0xa0
    strb r0, [r10, r12]
    add r0, r9, #16
    strb r0, [r10, #1]
    strh r0, [r11, #4]
    mov r0, #0
    bl select_bank
    bx r7

@ Bank selection is volatile protocol state; it never modifies backup bytes.
select_bank:
    mov r2, lr
    mov r3, r0
    ldr r12, =0x5555
    ldr r6, =0x2aaa
    bl unlock
    mov r0, #0xb0
    strb r0, [r10, r12]
    strb r3, [r10]
    bx r2

@ Every operation starts with the standard two-address unlock handshake.
unlock:
    mov r0, #0xaa
    strb r0, [r10, r12]
    mov r0, #0x55
    strb r0, [r10, r6]
    bx lr
    .ltorg
    .asciz "FLASH1M_V103"
    .balign 4
