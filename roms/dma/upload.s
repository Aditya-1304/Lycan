@ Original MIT DMA tile-upload scene. Firmware owns exception entry/return.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry, "ax", %progbits
.global _start
_start:
    ldr r0, =0x04000000
    mov r1, #0x80
    strh r1, [r0]
    ldr r4, =0x040000d4
    ldr r1, =red
    str r1, [r4]
    ldr r1, =0x06000000
    str r1, [r4, #4]
    ldr r1, =0x81008000       @ 32K halfwords, fixed source: bounded large upload.
    str r1, [r4, #8]
    @ The completed large upload contains a nonzero sentinel throughout VRAM.
    @ Clear only the screen map so every displayed entry selects the red tile 0.
    ldr r1, =zero
    str r1, [r4]
    ldr r1, =0x06008000
    str r1, [r4, #4]
    ldr r1, =0x81001000
    str r1, [r4, #8]
    ldr r1, =0x05000000
    mov r2, #0x1f
    strh r2, [r1, #2]
    ldr r2, =0x03e0
    strh r2, [r1, #4]
    mov r1, #0x1000          @ BG0 screen block 16; tile 0 repeats across the map.
    strh r1, [r0, #8]
    mov r1, #0x100
    strh r1, [r0]
    ldr r11, =0x03000000
    mov r1, #0x65
    strh r1, [r11]
    msr cpsr_c, #0xd2
    ldr sp, =0x03007fa0
    msr cpsr_c, #0xdf
    ldr r1, =upload_irq
    ldr r2, =0x03007ffc
    str r1, [r2]
    ldr r2, =0x04000200
    ldr r1, =0x0801          @ VBlank and DMA3 use the shared interrupt route.
    strh r1, [r2]
    mov r1, #1
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
upload_irq:
    ldr r0, =0x04000202
    ldrh r1, [r0]
    strh r1, [r11, #6]       @ Snapshot before normal write-one-to-clear acknowledgement.
    strh r1, [r0]
    ldrh r2, [r0]
    strh r2, [r11, #8]
    tst r1, #0x800
    beq no_completion
    ldrh r2, [r11, #4]
    add r2, r2, #1
    strh r2, [r11, #4]
no_completion:
    tst r1, #1
    bxeq lr
    ldrh r2, [r11, #2]
    add r2, r2, #1
    strh r2, [r11, #2]
    ldr r0, =0x040000d4
    ldr r2, =0x04000130
    ldrh r2, [r2]
    tst r2, #1
    ldrne r2, =red
    ldreq r2, =green
    str r2, [r0]
    ldr r2, =0x06000000
    str r2, [r0, #4]
    ldr r2, =0xd5000008      @ Word, fixed source, VBlank trigger, completion IRQ.
    str r2, [r0, #8]
    bx lr
    .ltorg
    .word 0, 0
.section .rodata, "a", %progbits
.balign 4
zero: .word 0
red: .word 0x11111111
green: .word 0x22222222
