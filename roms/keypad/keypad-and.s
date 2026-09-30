@ Original MIT guest: keypad requests use the mapped firmware IRQ vector.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
_start:
    msr cpsr_c, #0xd2
    ldr sp, =0x03007fa0
    msr cpsr_c, #0xdf
    ldr sp, =0x03007e00
    ldr r0, =0x03007ffc
    ldr r1, =callback
    str r1, [r0]
    ldr r0, =0x04000000
    mov r1, #0x400
    strh r1, [r0]
    ldr r1, =key_control
    ldr r1, [r1]
    ldr r2, =0x04000132
    strh r1, [r2]
    ldr r2, =0x04000200
    mov r1, #0x1000
    strh r1, [r2]
    mov r1, #1
    strh r1, [r2, #8]
    msr cpsr_c, #0x5f
    ldr r5, =0x03000000
    mov r1, #0x64
    strh r1, [r5]
    ldr r6, =0x04000301
    mov r1, #0
sleep:
    strb r1, [r6]
    ldrh r2, [r5, #8]
    add r2, r2, #1
    strh r2, [r5, #8]
    b sleep
    .ltorg
callback:
    @ This one-shot guest disables further keypad requests after the first wake.
    ldr r0, =0x04000132
    mov r1, #0
    strh r1, [r0]
    @ Record IF and acknowledgement through the ordinary hardware register path.
    ldr r0, =0x04000202
    ldrh r1, [r0]
    ldr r2, =0x03000000
    strh r1, [r2, #4]
    strh r1, [r0]
    ldrh r1, [r0]
    strh r1, [r2, #6]
    ldrh r1, [r2, #2]
    add r1, r1, #1
    strh r1, [r2, #2]
    @ Publish a conspicuous guest-owned result after recording IRQ completion.
    ldr r0, =0x05000000
    mov r1, #0x001f           @ Backdrop entry zero: full-intensity red.
    strh r1, [r0]
    bx lr
    .ltorg
.org 0x300
key_control:
    .word 0xc003
    .word 0, 0
