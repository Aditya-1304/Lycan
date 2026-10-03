@ Original MIT affine image demo. Firmware-free diagnostic startup only.
@ Each build selects one display mode; the same controls drive every image.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
_start:
    ldr r0, =0x04000000
    mov r1, #0x80
    strh r1, [r0]
    ldr r4, =0x05000000
    mov r2, #0
palette:
    orr r1, r2, r2, lsl #5
    strh r1, [r4], #2
    add r2, r2, #1
    cmp r2, #256
    bne palette
.if MODE < 3
    @ Tile 1 contains a unique 8x8 gradient; the 128x128 map repeats it.
    ldr r4, =0x06000040
    mov r2, #1
texels:
    add r1, r2, #1
    orr r1, r2, r1, lsl #8
    strh r1, [r4], #2
    add r2, r2, #2
    cmp r2, #65
    bne texels
    ldr r4, =0x06008000
    ldr r1, =0x0101
    mov r2, #128
map:
    strh r1, [r4], #2
    subs r2, r2, #1
    bne map
.else
    @ Coordinate gradients expose wrong source addressing, not just image bounds.
    ldr r4, =0x06000000
    mov r7, #0
    bl fill_bitmap
.if MODE != 3
    ldr r4, =0x0600a000
    mov r7, #1
    bl fill_bitmap
.endif
    b setup
fill_bitmap:
    mov r2, #0
bitmap_row:
    mov r3, #0
bitmap_pixel:
.if MODE == 4
    mov r1, r3, lsr #1
    add r1, r1, r2
    and r1, r1, #31
    add r1, r1, #1
    add r1, r1, r7, lsl #6
    orr r1, r1, r1, lsl #8
    add r3, r3, #2
.else
    and r1, r3, #31
    and r8, r2, #31
    orr r1, r1, r8, lsl #5
    orr r1, r1, r7, lsl #14
    add r3, r3, #1
.endif
    strh r1, [r4], #2
.if MODE == 5
    cmp r3, #160
.else
    cmp r3, #240
.endif
    bne bitmap_pixel
    add r2, r2, #1
.if MODE == 5
    cmp r2, #128
.else
    cmp r2, #160
.endif
    bne bitmap_row
    bx lr
setup:
.endif
    mov r5, #0                 @ Rotation: 0, 90, 180, 270 degrees.
    mov r6, #256               @ Signed 8.8 scale, either 1x or 2x source step.
    mov r7, #0                 @ Page select.
    mov r8, #0                 @ Wrap control.
    ldr r10, =0x04000130
    mov r11, #0
apply:
    ldr r4, =0x800
.if MODE == 2
    add r4, r4, #MODE
    add r9, r0, #0x30          @ Mode 2 exercises BG3; other modes use BG2.
    mov r3, #14
.else
    add r4, r4, #MODE
    sub r4, r4, #0x400
    add r9, r0, #0x20
    mov r3, #12
.endif
    orr r4, r4, r7
    strh r4, [r0]
    ldr r1, =0x1000
    orr r1, r1, r8
    strh r1, [r0, r3]
    mov r1, #0
    str r1, [r9]
    str r1, [r9, #4]
    str r1, [r9, #8]
    str r1, [r9, #12]
    cmp r5, #0
    beq identity
    cmp r5, #2
    beq reverse
    cmp r5, #1
    rsbeq r1, r6, #0
    moveq r2, r6
    movne r1, r6
    rsbne r2, r6, #0
    strh r1, [r9, #2]
    strh r2, [r9, #4]
    b ready
reverse:
    rsb r1, r6, #0
    strh r1, [r9]
    strh r1, [r9, #6]
    b ready
identity:
    strh r6, [r9]
    strh r6, [r9, #6]
ready:
    @ Center the transformed source on the LCD using integer reference coordinates.
    mov r1, r6, lsr #8
    mov r2, #120
    mul r2, r1, r2
    mov r3, #80
    mul r3, r1, r3
.if MODE < 3
    mov r4, #64
    mov r12, #64
.elseif MODE == 5
    mov r4, #80
    mov r12, #64
.else
    mov r4, #120
    mov r12, #80
.endif
    cmp r5, #0
    subeq r4, r4, r2
    subeq r12, r12, r3
    cmp r5, #1
    addeq r4, r4, r3
    subeq r12, r12, r2
    cmp r5, #2
    addeq r4, r4, r2
    addeq r12, r12, r3
    cmp r5, #3
    subeq r4, r4, r3
    addeq r12, r12, r2
    mov r4, r4, lsl #8
    mov r12, r12, lsl #8
    str r4, [r9, #8]
    str r12, [r9, #12]
    ldr r12, =0x03000000
    mov r1, #0xa1
    strh r1, [r12]
    strh r5, [r12, #2]
    strh r6, [r12, #4]
    strh r7, [r12, #6]
    strh r8, [r12, #8]
wait_visible:
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs wait_visible
wait_blank:
    ldrh r1, [r0, #6]
    cmp r1, #160
    blo wait_blank
    ldrh r2, [r10]
    mvn r2, r2
    bic r3, r2, r11            @ Apply controls once on each press edge.
    mov r11, r2
    tst r3, #0x10              @ Right rotates clockwise.
    addne r5, r5, #1
    and r5, r5, #3
    tst r3, #0x40              @ Up toggles scaling.
    eorne r6, r6, #0x300
    tst r3, #1                 @ A changes bitmap page.
    eorne r7, r7, #0x10
    tst r3, #2                 @ B toggles tiled wrapping.
    eorne r8, r8, #0x2000
    b apply
    .ltorg
    .space 8                 @ Keep literal fetch/prefetch words within the ROM image.
