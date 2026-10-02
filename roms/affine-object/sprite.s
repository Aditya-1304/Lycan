@ Original MIT interactive affine OBJ diagnostic; controlled ARM startup.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
_start:
    ldr r0, =0x04000000
    mov r1, #0x80
    strh r1, [r0]
    @ Disable unused OAM entries, including entries holding matrix coefficients.
    ldr r4, =0x07000000
    mov r1, #0x200
    mov r2, #128
disable:
    strh r1, [r4], #8
    subs r2, r2, #1
    bne disable
    @ Existing text-background path: tile 0 is an opaque green field, priority 1.
    ldr r4, =0x06000000
    ldr r1, =0x1111
    mov r2, #16
background:
    strh r1, [r4], #2
    subs r2, r2, #1
    bne background
    ldr r4, =0x05000000
    mov r1, #0x3e0
    strh r1, [r4, #2]
    ldr r1, =0x1001
    strh r1, [r0, #8]
    @ OBJ palette bank 2 and the 256-color palette share the source colors.
    ldr r4, =0x05000200
    mov r2, #1
palette:
    orr r1, r2, r2, lsl #10
    mov r3, r2, lsl #1
    strh r1, [r4, r3]
    add r3, r3, #64
    strh r1, [r4, r3]
    add r2, r2, #1
    cmp r2, #16
    bne palette
    mov r5, #0                 @ Matrix index: identity, 45, 90, 180 degrees.
    mov r6, #0                 @ Enlargement (source step halved).
    mov r7, #0                 @ Expanded bounds.
    mov r8, #0                 @ 256-color mode.
    mov r9, #0                 @ 2D tile mapping.
    mov r10, #0                @ Behind background.
    mov r11, #0                @ Wrapped top/left position.
    mov r12, #0                @ Previous keys.
apply:
    @ Repack the same asymmetric 32x32 source into the selected tile layout.
    ldr r4, =0x06010000
    ldr r1, =texture
    cmp r8, #0
    addeq r1, r1, #4096
    cmp r9, #0
    addne r1, r1, #8192
    mov r2, #2048
copy:
    ldrh r3, [r1], #2
    strh r3, [r4], #2
    subs r2, r2, #1
    bne copy
    @ Matrix 31 uses the final four OAM parameter slots; flip bits are index bits.
    ldr r4, =matrices
    add r4, r4, r5, lsl #3
    ldr r3, =0x070003e6
    mov r2, #4
matrix:
    ldrsh r1, [r4], #2
    cmp r6, #0
    movne r1, r1, asr #1
    strh r1, [r3], #8
    subs r2, r2, #1
    bne matrix
    ldr r4, =0x07000000
    mov r1, #60
    mov r2, #100
    cmp r11, #0
    movne r1, #248
    ldrne r2, =504
    cmp r7, #0
    subne r1, r1, #16
    subne r2, r2, #16
    and r1, r1, #255
    ldr r3, =511
    and r2, r2, r3
    orr r1, r1, #0x100
    orr r1, r1, r7, lsl #9
    orr r1, r1, r8, lsl #13
    strh r1, [r4]
    ldr r3, =0xbe00
    orr r2, r2, r3
    strh r2, [r4, #2]
    ldr r1, =0x2000
    orr r1, r1, r10, lsl #11
    strh r1, [r4, #4]
    ldr r1, =0x1140
    cmp r9, #0
    bicne r1, r1, #0x40
    strh r1, [r0]
    ldr r4, =0x03000000
    mov r1, #0xa2
    strh r1, [r4]
    strh r5, [r4, #2]
    strh r6, [r4, #4]
    strh r7, [r4, #6]
    strh r8, [r4, #8]
    strh r9, [r4, #10]
    strh r10, [r4, #12]
    strh r11, [r4, #14]
wait_visible:
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs wait_visible
wait_blank:
    ldrh r1, [r0, #6]
    cmp r1, #160
    blo wait_blank
    ldr r4, =0x04000130
    ldrh r2, [r4]
    mvn r2, r2
    bic r3, r2, r12
    mov r12, r2
    cmp r3, #0
    beq wait_visible
    tst r3, #16                @ Right: rotation.
    addne r5, r5, #1
    and r5, r5, #3
    tst r3, #64                @ Up: enlargement.
    eorne r6, r6, #1
    tst r3, #1                 @ A: expanded bounds.
    eorne r7, r7, #1
    tst r3, #2                 @ B: color depth.
    eorne r8, r8, #1
    tst r3, #256               @ R: tile mapping.
    eorne r9, r9, #1
    tst r3, #512               @ L: BG priority.
    eorne r10, r10, #1
    tst r3, #32                @ Left: wrap position across LCD origin.
    eorne r11, r11, #1
    b apply
    .ltorg
matrices:
    .hword 256,0,0,256, 181,-181,181,181, 0,-256,256,0, -256,0,0,-256
texture:
    .incbin "texture.bin"
    .space 8
