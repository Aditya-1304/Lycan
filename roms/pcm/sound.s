@ Original MIT synchronized PCM scene. One VBlank callback selects both the
@ square color and DMA1's fixed source; timers and FIFO playback stay hardware-owned.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry, "ax", %progbits
.global _start
_start:
    ldr r0, =0x04000000
    mov r1, #0x80
    strh r1, [r0, #0x84]   @ Enable the mixer before configuring Direct Sound.
    ldr r1, =0x0304
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
    @ Prime all 32 FIFO bytes before starting the 8192 Hz timer. The output core
    @ samples held FIFO levels at 32768 Hz; host adapters resample that stream.
    ldr r4, =0x040000a0
    mov r1, #0
    mov r2, #8
prime:
    str r1, [r4]
    subs r2, r2, #1
    bne prime
    ldr r4, =0x040000bc
    ldr r1, =silence
    str r1, [r4]
    ldr r1, =0x040000a0
    str r1, [r4, #4]
    ldr r1, =0xb7000001     @ FIFO repeat, fixed source; count is ignored in FIFO mode.
    str r1, [r4, #8]
    ldr r4, =0x04000100
    ldr r1, =0x0080f800     @ Timer 0 reload: 2048 master cycles per FIFO byte.
    str r1, [r4]
    ldr r11, =0x03000000
    mov r1, #0x66
    strh r1, [r11]
    msr cpsr_c, #0xd2
    ldr sp, =0x03007fa0
    msr cpsr_c, #0xdf
    ldr r1, =sound_irq
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
sound_irq:
    ldr r0, =0x04000202
    mov r1, #1
    strh r1, [r0]
    ldrh r1, [r11, #2]
    add r1, r1, #1
    strh r1, [r11, #2]       @ VBlank count proves the normal IRQ/HALT path ran.
    ldr r0, =0x04000130
    ldrh r1, [r0]
    @ Mixer controls are applied every VBlank, independently of the A-button
    @ transition. Down disables master; Left/Right route A; Select halves gain;
    @ Up selects 6-bit PWM with a raised bias; B strobes FIFO reset.
    mvn r3, r1
    ldr r0, =0x04000000
    tst r3, #0x80
    moveq r2, #0x80
    movne r2, #0
    strh r2, [r0, #0x84]
    ldr r2, =0x0304
    tst r3, #0x20
    movne r2, #0x0204
    tst r3, #0x10
    movne r2, #0x0104
    tst r3, #4
    bicne r2, r2, #4
    tst r3, #2
    orrne r2, r2, #0x800
    strh r2, [r0, #0x82]
    mov r2, #0x200
    tst r3, #0x40
    ldrne r2, =0xc240
    strh r2, [r0, #0x88]
    and r1, r1, #1
    eor r1, r1, #1
    ldrh r2, [r11, #4]
    cmp r1, r2
    bxeq lr
    strh r1, [r11, #4]
    ldrh r2, [r11, #6]
    add r2, r2, #1
    strh r2, [r11, #6]       @ Count transitions rather than held-frame repetitions.
    ldr r0, =0x05000002
    cmp r1, #0
    moveq r2, #0x1f
    ldrne r2, =0x03e0
    strh r2, [r0]
    ldr r0, =0x040000bc
    mov r2, #0
    strh r2, [r0, #10]      @ Disable before relatching a different DMA source.
    ldreq r2, =silence
    ldrne r2, =tone
    str r2, [r0]
    ldr r2, =0xb700
    strh r2, [r0, #10]
    bx lr
    .ltorg
    .word 0, 0
.section .rodata, "a", %progbits
.balign 4
tile: .word 0x11111111
silence: .word 0
tone: .word 0xe0e02020      @ Signed +32,+32,-32,-32: deterministic 2048 Hz tone.
