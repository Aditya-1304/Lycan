@ Original MIT wave effect with pulse accompaniment and DMA PCM. One VBlank callback selects both the
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
    ldr r1, =0x0306
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
    ldr r1, =0xffff
    strh r1, [r11, #8]
    strh r1, [r11, #14]
    mov r1, #0x77
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
    push {lr}
    bl pulse_update
    bl wave_update
    pop {lr}
    ldr r0, =0x04000202
    mov r1, #1
    strh r1, [r0]
    ldrh r1, [r11, #2]
    add r1, r1, #1
    strh r1, [r11, #2]       @ VBlank count proves the normal IRQ/HALT path ran.
    ldr r0, =0x04000130
    ldrh r1, [r0]
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
    @ PCM input changes audio only; note color remains visible.
    ldr r0, =0x040000bc
    mov r2, #0
    strh r2, [r0, #10]      @ Disable before relatching a different DMA source.
    ldreq r2, =silence
    ldrne r2, =tone
    str r2, [r0]
    ldr r2, =0xb700
    strh r2, [r0, #10]
    bx lr
    @ Pulse controls are latched only on a note or key transition; retriggering
    @ every VBlank would prevent the envelope and sweep from progressing.
pulse_update:
    ldr r0, =0x04000130
    ldrh r1, [r0]
    mvn r1, r1
    and r1, r1, #0xc6
    ldrh r2, [r11, #2]
    mov r2, r2, lsr #5
    and r2, r2, #3
    orr r2, r1, r2, lsl #8
    ldrh r3, [r11, #8]
    cmp r2, r3
    bxeq lr
    strh r2, [r11, #8]
    ldr r0, =0x04000000
    tst r1, #0x80
    movne r3, #0
    strhne r3, [r0, #0x84]
    bxne lr
    mov r3, #0x80
    strh r3, [r0, #0x84]
    ldr r3, =0x3477       @ Pulse accompaniment left, wave right; full PSG volume.
    strh r3, [r0, #0x80]
    mov r3, #8
    tst r1, #2
    movne r3, #0x29       @ B: decreasing sweep, period 2, shift 1.
    strh r3, [r0, #0x60]
    ldr r3, =0x4080       @ Lead: 50% duty, constant volume 4.
    tst r1, #4
    ldrne r3, =0x4040     @ Select: 25% duty.
    tst r1, #0x40
    orrne r3, r3, #0x100  @ Up: decreasing envelope, one 64 Hz tick per step.
    strh r3, [r0, #0x62]
    ldr r3, =0x2080       @ Accompaniment: 50% duty, constant volume 2.
    strh r3, [r0, #0x68]
    mov r2, r2, lsr #8
    mov r2, r2, lsl #1
    ldr r3, =notes
    ldrh r3, [r3, r2]
    strh r3, [r11, #10]   @ Source frequency is observable despite write-only IO.
    orr r3, r3, #0x8000
    strh r3, [r0, #0x64]
    ldr r3, =harmony
    ldrh r3, [r3, r2]
    strh r3, [r11, #12]
    orr r3, r3, #0x8000
    strh r3, [r0, #0x6c]
    ldr r0, =0x05000002
    ldr r3, =colors
    ldrh r3, [r3, r2]
    strh r3, [r0]         @ The visible square identifies the current note.
    bx lr
    @ Effects are rewritten only on input changes. The inactive bank remains
    @ CPU-accessible while the selected bank feeds the wave oscillator.
wave_update:
    ldr r0, =0x04000130
    ldrh r1, [r0]
    mvn r1, r1
    and r1, r1, #0xc7
    ldrh r2, [r11, #14]
    cmp r1, r2
    bxeq lr
    strh r1, [r11, #14]
    tst r1, #0x80
    bxne lr
    ldr r0, =0x04000000
    mov r2, #0x40
    strh r2, [r0, #0x70]  @ Select bank 1 so writes fill bank 0.
    ldr r3, =0xf0f0
    tst r1, #1
    ldrne r3, =0xff00      @ A selects a second waveform and existing PCM.
    mov r2, #0x90
wave_fill_first:
    strh r3, [r0, r2]
    add r2, r2, #2
    cmp r2, #0xa0
    bne wave_fill_first
    mov r2, #0
    strh r2, [r0, #0x70]  @ Select bank 0 so writes fill bank 1.
    ldr r3, =0x00ff
    mov r2, #0x90
wave_fill_second:
    strh r3, [r0, r2]
    add r2, r2, #2
    cmp r2, #0xa0
    bne wave_fill_second
    mov r2, #0x80
    tst r1, #2
    orrne r2, r2, #0x40   @ B starts playback from bank 1.
    tst r1, #4
    orrne r2, r2, #0x20   @ Select plays both 32-digit banks.
    strh r2, [r0, #0x70]
    mov r2, #0x2000
    tst r1, #0x40
    movne r2, #0x8000     @ Up forces 75% gain.
    strh r2, [r0, #0x72]
    ldr r2, =0x8780       @ 1024 cycles/digit; restart self-clears.
    strb r2, [r0, #0x74]
    mov r2, r2, lsr #8
    strb r2, [r0, #0x75]  @ High-byte trigger retains write-only frequency bits.
    bx lr
    .ltorg
    .word 0, 0
.section .rodata, "a", %progbits
.balign 4
tile: .word 0x11111111
silence: .word 0
tone: .word 0xe0e02020      @ Signed +32,+32,-32,-32: deterministic 2048 Hz tone.

notes: .hword 1536, 1642, 1707, 1792
harmony: .hword 1024, 1236, 1366, 1536
colors: .hword 31, 992, 31744, 32767
