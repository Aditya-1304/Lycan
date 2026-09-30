@ Original MIT sprite scene. The guest owns OAM, tile data, input and BG scroll.
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

    ldr r4, =0x07000000         @ Disable every object before selecting OAM entries.
    mov r1, #0x200
    mov r5, #128
clear_oam:
    strh r1, [r4], #8
    subs r5, r5, #1
    bne clear_oam
    ldr r4, =0x05000260         @ OBJ palette bank 3, independent of BG palettes.
    mov r1, #0x1f
    strh r1, [r4, #2]
    ldr r1, =0x03e0
    strh r1, [r4, #4]
    ldr r1, =0x7c00
    strh r1, [r4, #6]
    ldr r1, =0x03ff
    strh r1, [r4, #8]
    ldr r1, =0x7c1f
    strh r1, [r4, #10]
    ldr r4, =object_tiles
    ldr r5, =0x06010080         @ 1D tile numbers 4..7.
    mov r6, #32
copy_object_tiles:
    ldr r1, [r4], #4
    str r1, [r5], #4
    subs r6, r6, #1
    bne copy_object_tiles
    ldr r4, =object_2d_bottom
    ldr r5, =0x06010480         @ 2D bottom row uses tile numbers 36..37.
    mov r6, #16
copy_object_2d:
    ldr r1, [r4], #4
    str r1, [r5], #4
    subs r6, r6, #1
    bne copy_object_2d
    ldr r4, =0x06010500         @ Solid overlap object: tiles 40..43, and 72..73.
    ldr r1, =0x55555555
    mov r5, #32
marker_tiles:
    str r1, [r4], #4
    subs r5, r5, #1
    bne marker_tiles
    ldr r4, =0x06010900
    mov r5, #16
marker_2d:
    str r1, [r4], #4
    subs r5, r5, #1
    bne marker_2d
    ldr r4, =0x07000000
    mov r5, #112                @ Player top-left (112,72), normal 16x16 4bpp.
    mov r6, #72
    strh r6, [r4]
    ldr r1, =0x4000
    orr r1, r1, r5
    strh r1, [r4, #2]
    ldr r1, =0x3004             @ Tile 4, OBJ bank 3, priority 0.
    strh r1, [r4, #4]
    mov r1, #80
    strh r1, [r4, #8]
    ldr r1, =0x4078             @ Overlap marker at (120,80), lower OAM precedence.
    strh r1, [r4, #10]
    ldr r1, =0x3028
    strh r1, [r4, #12]
    ldr r2, =510
    mov r3, r2
    ldr r9, =511
    ldr r10, =0x04000130
    ldr r11, =0x03000000        @ Mailbox: ID, x, y, completed scroll-update count.
    mov r12, #0
    mov r1, #0x62
    strh r1, [r11]
    strh r2, [r0, #0x10]
    strh r3, [r0, #0x12]
    strh r2, [r11, #2]
    strh r3, [r11, #4]
    strh r12, [r11, #6]
    strh r5, [r11, #8]
    strh r6, [r11, #10]
    mov r1, #0
    strh r1, [r11, #12]         @ OBJ priority and layout are frozen observations.
    mov r1, #1
    strh r1, [r11, #14]
    ldr r1, =0x1140             @ BG0 + OBJ enabled, initial 1D object mapping.
    strh r1, [r0]
.ifdef VBLANK_IRQ
    @ The scene supplies its own IRQ stack and the standard firmware callback.
    msr cpsr_c, #0xd2
    ldr sp, =0x03007fa0
    msr cpsr_c, #0xdf
    ldr r1, =scene_irq
    ldr r4, =0x03007ffc
    str r1, [r4]
    add r4, r0, #0x200
    mov r1, #1
    strh r1, [r4]
    strh r1, [r4, #8]
    mov r1, #8
    strh r1, [r0, #4]
    msr cpsr_c, #0x5f
.endif
frame:
.ifdef VBLANK_IRQ
    ldr r4, =0x04000301
    mov r1, #0
    strb r1, [r4]              @ VBlank dispatch drives the existing scene update.
.else
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs frame
wait_vblank:
    ldrh r1, [r0, #4]
    tst r1, #1
    beq wait_vblank
.endif
    ldrh r8, [r10]
    tst r8, #0x10
    addeq r2, r2, #2
    addeq r5, r5, #2
    tst r8, #0x20
    subeq r2, r2, #2
    subeq r5, r5, #2
    tst r8, #0x80
    addeq r3, r3, #2
    addeq r6, r6, #2
    tst r8, #0x40
    subeq r3, r3, #2
    subeq r6, r6, #2
    cmp r5, #0
    movlt r5, #0
    cmp r5, #224
    movgt r5, #224
    cmp r6, #0
    movlt r6, #0
    cmp r6, #144
    movgt r6, #144
    ldr r4, =0x07000000
    strh r6, [r4]
    orr r1, r5, #0x4000
    strh r1, [r4, #2]
    ldr r1, =0x3004
    tst r8, #1                 @ A (Z) moves player behind BG0, preserving OBJ order.
    orreq r1, r1, #0x400
    strh r1, [r4, #4]
    mov r7, r1, lsr #10
    and r7, r7, #3
    strh r7, [r11, #12]
    ldr r1, =0x1100
    tst r8, #2                 @ B (X) selects 2D; release selects 1D.
    orrne r1, r1, #0x40
    strh r1, [r0]
    mov r7, r1, lsr #6
    and r7, r7, #1
    strh r7, [r11, #14]
    strh r5, [r11, #8]
    strh r6, [r11, #10]
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
.ifdef VBLANK_IRQ
scene_irq:
    @ Save/restore of volatile registers and exception return belong to firmware.
    ldr r0, =0x04000202
    ldrh r1, [r0]
    strh r1, [r11, #20]
    strh r1, [r0]
    ldrh r1, [r0]
    strh r1, [r11, #22]
    ldrh r1, [r11, #16]
    add r1, r1, #1
    strh r1, [r11, #16]        @ IRQ count is independent of main-loop update count.
    bx lr
    .ltorg
.endif
    .word 0, 0
.size _start, . - _start

.section .rodata, "a", %progbits
.balign 4
@ Each 8x8 tile has one transparent external edge. The four colors and asymmetric
@ border expose wrong tile strides, OBJ palette addressing and texel transparency.
.macro object_tile color, left, right, top, bottom
.irp row, 0,1,2,3,4,5,6,7
.if (\top && \row == 0) || (\bottom && \row == 7)
    .word 0
.else
    .word (\color * 0x11111111) & (0xffffffff ^ (\left * 15) ^ (\right * 0xf0000000))
.endif
.endr
.endm
object_tiles:
    object_tile 1, 1, 0, 1, 0
    object_tile 2, 0, 1, 1, 0
    object_tile 3, 1, 0, 0, 1
    object_tile 4, 0, 1, 0, 1

@ 2D mapping deliberately swaps bottom colors. Retaining the 1D tile stride
@ therefore produces a different image and cannot satisfy the frozen capture.
object_2d_bottom:
    object_tile 4, 1, 0, 0, 1
    object_tile 3, 0, 1, 0, 1
