@ Original MIT noise scene. Paired visible events retrigger channel four and
@ swap FIFO timer selection; hardware owns cascades, DMA refill and synthesis.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry, "ax", %progbits
.global _start
_start:
    ldr r0, =0x04000000
    mov r1, #0x80
    strh r1, [r0, #0x84]   @ Enable the mixer before configuring Direct Sound.
    ldr r1, =0x520e
    strh r1, [r0, #0x82]   @ FIFO A, full volume, both speakers, timer 0.
    mov r1, #0x200
    strh r1, [r0, #0x88]   @ Centered 9-bit / 32768 Hz PWM output.
    mov r1, #0x80
    strh r1, [r0]
    @ Tile 1 is an opaque palette-1 square; tile 0 and the remaining map stay empty.
    ldr r4, =0x040000d4
    ldr r1, =tile
    str r1, [r4]
    ldr r1, =0x06000020
    str r1, [r4, #4]
    ldr r1, =0x81000010
    str r1, [r4, #8]
    ldr r1, =0x0600825c
    mov r2, #1
    strh r2, [r1]
    strh r2, [r1, #2]
    strh r2, [r1, #64]
    strh r2, [r1, #66]
    ldr r1, =0x05000002
    mov r2, #0x1f
    strh r2, [r1]
    mov r1, #0x1000
    strh r1, [r0, #8]
    mov r1, #0x100
    strh r1, [r0]
    @ Both pulse voices and the wave oscillator remain active under each effect.
    ldr r1, =0xbf77
    strh r1, [r0, #0x80]
    ldr r1, =0x2080
    strh r1, [r0, #0x62]
    ldr r1, =0x8600
    strh r1, [r0, #0x64]
    ldr r1, =0x1080
    strh r1, [r0, #0x68]
    ldr r1, =0x8400
    strh r1, [r0, #0x6c]
    mov r1, #0x40
    strh r1, [r0, #0x70]
    ldr r1, =0xf0f0
    mov r2, #0x90
wave_fill:
    strh r1, [r0, r2]
    add r2, r2, #2
    cmp r2, #0xa0
    bne wave_fill
    mov r1, #0x80
    strh r1, [r0, #0x70]
    mov r1, #0x6000
    strh r1, [r0, #0x72]
    ldr r1, =0x8780
    strh r1, [r0, #0x74]
    @ Prime both FIFOs before enabling the complete timer chain. Fixed-source
    @ descriptors give deterministic PCM indefinitely without source exhaustion.
    ldr r4, =0x040000a0
    ldr r1, =0xf8f80808
    ldr r3, =0xf0f01010
    mov r2, #8
prime:
    str r1, [r4]
    str r3, [r4, #4]
    subs r2, r2, #1
    bne prime
    ldr r4, =0x040000bc
    ldr r1, =pcm_a
    str r1, [r4]
    ldr r1, =0x040000a0
    str r1, [r4, #4]
    ldr r1, =0xb3000001     @ Hardware forces words/count/fixed destination.
    str r1, [r4, #8]
    ldr r4, =0x040000c8
    ldr r1, =pcm_b
    str r1, [r4]
    ldr r1, =0x040000a4
    str r1, [r4, #4]
    ldr r1, =0xb3000001
    str r1, [r4, #8]
    ldr r4, =0x04000100
    ldr r1, =0x00c4fffc
    str r1, [r4, #12]     @ Timer 3: cascade /4, interrupt flag enabled.
    ldr r1, =0x00c4fffd
    str r1, [r4, #8]      @ Timer 2: cascade /3.
    ldr r1, =0x00c4fffe
    str r1, [r4, #4]      @ Timer 1: cascade /2; alternate FIFO clock.
    ldr r1, =0x00c0f800
    str r1, [r4]          @ Timer 0: master /2048, starts the entire chain.
    ldr r11, =0x03000000
    ldr r1, =0xffff
    strh r1, [r11, #4]
    mov r1, #0x79
    strh r1, [r11]
    msr cpsr_c, #0xd2
    ldr sp, =0x03007fa0
    msr cpsr_c, #0xdf
    ldr r1, =scene_irq
    ldr r2, =0x03007ffc
    str r1, [r2]
    ldr r2, =0x04000200
    mov r1, #1
    strh r1, [r2]
    strh r1, [r2, #8]
    mov r1, #8
    strh r1, [r0, #4]
    msr cpsr_c, #0x5f
.global ready
ready:
    ldr r0, =0x04000301
    mov r1, #0
    strb r1, [r0]
    b ready
scene_irq:
    ldrh r2, [r11, #2]
    add r2, r2, #1
    strh r2, [r11, #2]
    ldr r0, =0x04000130
    ldrh r1, [r0]
    mvn r1, r1
    and r1, r1, #7
    mov r2, r2, lsr #5
    and r2, r2, #1
    orr r2, r1, r2, lsl #8
    ldrh r3, [r11, #4]
    cmp r2, r3
    beq acknowledge
    strh r2, [r11, #4]
    ldrh r3, [r11, #6]
    add r3, r3, #1
    strh r3, [r11, #6]
    @ A toggles the automatic event phase; B selects the seven-stage effect.
    eor r2, r2, r2, lsr #8
    and r2, r2, #1
    ldr r0, =0x04000000
    cmp r2, #0
    ldreq r3, =0x520e     @ FIFO A timer 0 left; FIFO B timer 1 right.
    ldrne r3, =0x160e     @ FIFO A timer 1 left; FIFO B timer 0 right.
    tst r1, #4
    movne r3, #0x0e       @ Select isolates noise; FIFOs continue consuming.
    strh r3, [r0, #0x82]
    strh r3, [r11, #8]
    ldr r3, =0xbf77
    tst r1, #4
    ldrne r3, =0x8877
    strh r3, [r0, #0x80]
    ldr r3, =0xa100       @ Volume 10, decreasing envelope, 64 length ticks.
    tst r1, #2
    ldrne r3, =0xa400       @ Slower fade exposes the short-mode period.
    strh r3, [r0, #0x78]
    mov r3, #0x44         @ Divisor 4 / shift 4: 4096 cycles per noise edge.
    tst r1, #2
    orrne r3, r3, #8
    strb r3, [r0, #0x7c]
    mov r3, #0xc0
    strb r3, [r0, #0x7d]  @ Byte trigger must preserve the frequency latch.
    ldr r0, =0x05000002
    cmp r2, #0
    moveq r3, #0x1f
    ldrne r3, =0x03e0
    strh r3, [r0]         @ Scene and both clock selections change together.
acknowledge:
    ldr r0, =0x04000202
    mov r1, #1
    strh r1, [r0]         @ Retain timer IF flags for independent verification.
    bx lr
    .ltorg
.section .rodata, "a", %progbits
.balign 4
tile: .word 0x11111111
pcm_a: .word 0xf8f80808
pcm_b: .word 0xf0f01010
