.syntax unified
.cpu arm7tdmi
.arm

@ Original MIT controlled-startup counter. ARM owns scanout and drawing;
@ Thumb reads KEYINPUT and increments the counter once per held-A frame.
.section .text.entry, "ax", %progbits
.global _start
_start:
    ldr r10, =0x04000000
    ldr r11, =0x03000000
    ldr r1, =0x0403
    strh r1, [r10]
    mov r8, #0
    mov r12, #0
    bl draw
frame:
    ldrh r1, [r10, #6]
    cmp r1, #160
    bhs frame
wait_blank:
    ldrh r1, [r10, #4]
    tst r1, #1
    beq wait_blank
    ldr r0, =thumb_counter
    adr lr, returned
    bx r0
.global returned
returned:
    @ These stores can execute only after the Thumb routine returns to ARM.
    strh r0, [r11, #2]
    str r2, [r11, #8]           @ ADR result, independently frozen in the manifest.
    str r4, [r11, #12]          @ Thumb architectural PC observed by MOV.
    add r12, r12, #1
    strh r12, [r11, #4]
    mov r1, #0x5f
    strh r1, [r11]
    bl draw
    b frame

@ Sixteen eight-pixel cells encode 0..16 as a green horizontal bar at y=72.
@ The bounded 128x16 redraw fits inside VBlank using the existing ARM subset.
draw:
    ldr r7, =0x06008770
    mov r6, r8, lsl #3
    mov r5, #16
row:
    mov r3, #0
pixel:
    cmp r3, r6
    movlo r1, #0x03e0
    movhs r1, #0
    strh r1, [r7], #2
    add r3, r3, #1
    cmp r3, #128
    blo pixel
    add r7, r7, #224
    subs r5, r5, #1
    bne row
    bx lr
    .ltorg

.thumb
.balign 4
.global thumb_counter
.thumb_func
thumb_counter:
    mov r9, lr                 @ Preserve the even ARM return address across BL.
    adr r2, keypad_address     @ Located at address 2 mod 4: ADR must align PC+4.
.global observed_pc
observed_pc:
    mov r4, pc
    ldr r3, [r2]
    ldrh r1, [r3]
    lsls r1, r1, #31           @ Active-low A is bit zero of KEYINPUT.
    bne unchanged
    bl increment
unchanged:
    mov r0, r8
    bx r9
.thumb_func
increment:
    movs r1, #16
    cmp r8, r1
    bhs full
    movs r0, #1
    add r8, r0
full:
    bx lr                     @ Odd Thumb link address retains Thumb state.
.balign 4
.global keypad_address
keypad_address:
    .word 0x04000130
    @ Pipeline lookahead remains mapped beyond both code and literal data.
    .word 0, 0
