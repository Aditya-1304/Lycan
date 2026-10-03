@ Original MIT guest: each VBlank callback changes the visible backdrop.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
_start:
    @ IRQ and system stacks are initialized by the guest, not callback dispatch.
    msr cpsr_c, #0xd2
    ldr sp, =0x03007fa0
    msr cpsr_c, #0xdf
    ldr sp, =0x03007e00
    ldr r0, =0x03007ffc
    ldr r1, =callback
    str r1, [r0]
    ldr r0, =0x04000000
    mov r1, #0x400
    strh r1, [r0]             @ Mode 0 backdrop; zeroed VRAM is transparent.
    ldr r4, =configuration
    ldr r4, [r4]
    tst r4, #1
    mov r1, #0
    movne r1, #8
    strh r1, [r0, #4]         @ Configuration bit 0 enables the VBlank source.
    add r3, r0, #0x200
    mov r1, #1
    strh r1, [r3]     @ IE selects VBlank independently of IME/CPSR.
    tst r4, #2
    mov r1, #0
    movne r1, #1
    strh r1, [r3, #8]
    tst r4, #4
    msrne cpsr_c, #0x5f       @ Configuration bit 2 unmasks CPU IRQ delivery.
    ldr r5, =0x03000000
    mov r1, #0x63
    strh r1, [r5]
    mov r1, #0
    ldr r6, =0x04000301
    tst r4, #8
    ldrne r3, =thumb_sleep + 1
    bxne r3
sleep:
    strb r1, [r6]             @ Only an enabled pending hardware source wakes HALT.
    ldrh r2, [r5, #8]
    add r2, r2, #1
    strh r2, [r5, #8]         @ Records wake even when IRQ delivery is masked.
    ldr r3, =0x04000202
    mov r2, #1
    strh r2, [r3]             @ Masked delivery still needs acknowledgement after wake.
    b sleep
    .ltorg

callback:
    @ Firmware preserves r0-r3/r12/LR; this callback does not use other registers.
    ldr r0, =0x04000202
    ldrh r1, [r0]
    ldr r2, =0x03000000
    strh r1, [r2, #4]         @ Observe the pending VBlank flag before W1C.
    strh r1, [r0]
    ldrh r1, [r0]
    strh r1, [r2, #6]         @ Acknowledgement must clear, rather than set, IF.
    ldrh r1, [r2, #2]
    add r1, r1, #1
    strh r1, [r2, #2]
    and r1, r1, #31
    ldr r0, =0x05000000
    strh r1, [r0]             @ Next scanout displays the callback count in red.
    bx lr
    .ltorg
@ The alternate loop proves IRQ return restores Thumb state and its next PC.
.thumb
.balign 2
thumb_sleep:
    strb r1, [r6]
    ldrh r2, [r5, #8]
    adds r2, #1
    strh r2, [r5, #8]
    ldr r3, =0x04000202
    movs r2, #1
    strh r2, [r3]
    b thumb_sleep
    .ltorg
.arm
.org 0x300
configuration:
    .word 7
    .word 0, 0               @ Map instruction prefetch beyond the configuration.
