@ Original MIT mode-0 scene. Tile/map/palette data and scrolling are guest owned.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry, "ax", %progbits
.global _start
.type _start, %function
_start:
    ldr r0, =0x04000000
    mov r1, #0x80               @ Keep setup hidden until all scene data is ready.
    strh r1, [r0]
    ldr r1, =0xd000             @ BG0: 512x512, screen base 16, 4bpp, character base 0.
    strh r1, [r0, #8]
    ldr r4, =0x05000000
    ldr r1, =0x7fff             @ Transparent texels reveal the white backdrop.
    strh r1, [r4]
    mov r5, #0
palette_bank:
    add r6, r5, #1
    mov r6, r6, lsl #8
    mov r7, #1
palette_color:
    add r1, r7, r7, lsl #5
    add r1, r1, r6
    add r8, r4, r7, lsl #1
    strh r1, [r8]
    add r7, r7, #1
    cmp r7, #8
    bne palette_color
    add r4, r4, #32
    add r5, r5, #1
    cmp r5, #4
    bne palette_bank

    ldr r4, =0x06000020         @ Tile 1 has columns 0..7; tile 2 has rows 0..7.
    ldr r1, =0x76543210
    mov r5, #8
column_tile:
    str r1, [r4], #4
    subs r5, r5, #1
    bne column_tile
    mov r5, #0
row_tile:
    mov r1, r5
    orr r1, r1, r1, lsl #4
    orr r1, r1, r1, lsl #8
    orr r1, r1, r1, lsl #16
    str r1, [r4], #4
    add r5, r5, #1
    cmp r5, #8
    bne row_tile

    ldr r4, =0x06008000         @ Four 32x32 screen blocks in row-major order.
    mov r5, #0
map_block:
    mov r6, #0
map_entry:
    and r7, r6, #31
    mov r8, r6, lsr #5
    add r1, r7, r8
    and r1, r1, #1
    add r1, r1, #1
    and r9, r7, #1
    orr r1, r1, r9, lsl #10     @ Alternate horizontal and vertical flips.
    and r9, r8, #1
    orr r1, r1, r9, lsl #11
    orr r1, r1, r5, lsl #12     @ Distinct palette bank for each map quadrant.
    strh r1, [r4], #2
    add r6, r6, #1
    cmp r6, #1024
    bne map_entry
    add r5, r5, #1
    cmp r5, #4
    bne map_block

    ldr r2, =510
    mov r3, r2
    ldr r9, =511
    ldr r10, =0x04000130
    ldr r11, =0x03000000        @ Mailbox: ID, x, y, completed scroll-update count.
    mov r12, #0
    mov r1, #0x61
    strh r1, [r11]
    strh r2, [r0, #0x10]
    strh r3, [r0, #0x12]
    strh r2, [r11, #2]
    strh r3, [r11, #4]
    strh r12, [r11, #6]
    mov r1, #0x100
    strh r1, [r0]
frame:
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs frame
wait_vblank:
    ldrh r1, [r0, #4]
    tst r1, #1
    beq wait_vblank
    ldrh r8, [r10]
    tst r8, #0x10
    addeq r2, r2, #2
    tst r8, #0x20
    subeq r2, r2, #2
    tst r8, #0x80
    addeq r3, r3, #2
    tst r8, #0x40
    subeq r3, r3, #2
    and r2, r2, r9
    and r3, r3, r9
    strh r2, [r0, #0x10]        @ Exactly one scroll update per VBlank edge.
    strh r3, [r0, #0x12]
    add r12, r12, #1
    strh r2, [r11, #2]
    strh r3, [r11, #4]
    strh r12, [r11, #6]
    b frame
    .ltorg
    .word 0, 0
.size _start, . - _start
