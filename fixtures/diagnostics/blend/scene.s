@ Original MIT Slice-24 color-effects diagnostic.
@ Controlled ARM startup, no BIOS and no cartridge backup.
.syntax unified
.cpu arm7tdmi
.arm

.section .text.entry,"ax",%progbits
.global _start

_start:
    ldr r0, =0x04000000

    @ Forced blank while constructing the scene.
    mov r1, #0x80
    strh r1, [r0]

    @ Disable all OAM entries.
    ldr r4, =0x07000000
    mov r1, #0x200
    mov r2, #128

clear_oam:
    strh r1, [r4], #8
    subs r2, r2, #1
    bne clear_oam

    @ ------------------------------------------------------------
    @ BG0 tile 0: red / transparent checkerboard.
    @ ------------------------------------------------------------
    ldr r4, =0x06000000
    ldr r1, =0x01010101
    ldr r2, =0x10101010
    mov r3, #4

bg0_tile:
    str r1, [r4], #4
    str r2, [r4], #4
    subs r3, r3, #1
    bne bg0_tile

    @ BG1 tile 1: solid palette index 2 (blue).
    ldr r1, =0x22222222
    mov r2, #8

bg1_tile:
    str r1, [r4], #4
    subs r2, r2, #1
    bne bg1_tile

    @ BG0 map: tile 0.
    ldr r4, =0x06008000
    mov r1, #0
    mov r2, #1024

bg0_map:
    strh r1, [r4], #2
    subs r2, r2, #1
    bne bg0_map

    @ BG1 map: tile 1.
    ldr r4, =0x06008800
    mov r1, #1
    mov r2, #1024

bg1_map:
    strh r1, [r4], #2
    subs r2, r2, #1
    bne bg1_map

    @ BG palette:
    @ 0 = black backdrop
    @ 1 = red
    @ 2 = blue
    ldr r4, =0x05000000
    mov r1, #0
    strh r1, [r4]

    mov r1, #31
    strh r1, [r4, #2]

    ldr r1, =0x7c00
    strh r1, [r4, #4]

    @ OBJ palette index 1 = white.
    ldr r4, =0x05000200
    ldr r1, =0x7fff
    strh r1, [r4, #2]

    @ OBJ tile 0: alternating opaque white / transparent pixels.
    ldr r4, =0x06010000
    ldr r1, =0x01010101
    mov r2, #8

obj_tile:
    str r1, [r4], #4
    subs r2, r2, #1
    bne obj_tile

    @ OBJ0: 8x8 at (97,72), priority 0, tile 0.
    @ X=97 deliberately crosses WIN0's X=100 boundary.
    ldr r4, =0x07000000
    mov r1, #72
    strh r1, [r4]

    mov r1, #97
    strh r1, [r4, #2]

    mov r1, #0
    strh r1, [r4, #4]

    @ BG0: priority 0, screen block 16.
    ldr r1, =0x1000
    strh r1, [r0, #8]

    @ BG1: priority 1, screen block 17.
    ldr r1, =0x1101
    strh r1, [r0, #10]

    @ WIN0: X=100..159, Y=40..119.
    ldr r1, =0x64a0
    strh r1, [r0, #0x40]

    ldr r1, =0x2878
    strh r1, [r0, #0x44]

    @ Interactive state and previous keypad sample.
    mov r5, #0
    mov r8, #0

apply:
    @ Common coefficients: 8/16 + 8/16 and EVY=8/16.
    ldr r1, =0x0808
    strh r1, [r0, #0x52]

    mov r1, #8
    strh r1, [r0, #0x54]

    @ Outside WIN0: BG0 + BG1 + OBJ + color effects.
    mov r1, #0x33
    strh r1, [r0, #0x4a]

    @ WIN0 defaults to the same permissions.
    mov r1, #0x33

    @ State 5: ordinary alpha disabled inside WIN0.
    cmp r5, #5
    moveq r1, #0x13

    @ State 6: BG1 hidden inside WIN0, effects remain enabled.
    cmp r5, #6
    moveq r1, #0x31

    @ State 7: all visual layers enabled but SFX disabled inside.
    cmp r5, #7
    moveq r1, #0x13

    strh r1, [r0, #0x48]

    @ ------------------------------------------------------------
    @ BLDCNT state
    @ ------------------------------------------------------------
    mov r1, #0

    @ 1: BG0 alpha first target, BG1 second target.
    cmp r5, #1
    ldreq r1, =0x0241

    @ 2: brighten BG0.
    cmp r5, #2
    moveq r1, #0x81

    @ 3: darken BG0.
    cmp r5, #3
    moveq r1, #0xc1

    @ 4: semitrans OBJ while BLDCNT asks to darken OBJ.
    @    Valid BG second target must force alpha instead.
    cmp r5, #4
    ldreq r1, =0x03d0

    @ 5: regular BG0/BG1 alpha, window bit gates it.
    cmp r5, #5
    ldreq r1, =0x0241

    @ 6: semitrans OBJ; WIN0 hides BG1.
    cmp r5, #6
    ldreq r1, =0x03d0

    @ 7: semitrans OBJ; WIN0 clears SFX bit.
    cmp r5, #7
    ldreq r1, =0x03d0

    strh r1, [r0, #0x50]

    @ ------------------------------------------------------------
    @ OBJ mode
    @ ------------------------------------------------------------
    mov r1, #72

    cmp r5, #4
    orreq r1, r1, #0x400

    cmp r5, #6
    orreq r1, r1, #0x400

    cmp r5, #7
    orreq r1, r1, #0x400

    ldr r4, =0x07000000
    strh r1, [r4]

    @ Mode 0, OBJ 1D mapping, BG0, BG1, OBJ, WIN0.
    ldr r1, =0x3340
    strh r1, [r0]

    @ Completion/state mailbox.
    ldr r4, =0x03000000
    mov r1, #0xa5
    strh r1, [r4]
    strh r5, [r4, #2]

wait_visible:
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs wait_visible

wait_blank:
    ldrh r1, [r0, #6]
    cmp r1, #160
    blo wait_blank

    @ Read active-low KEYINPUT and keep only ten keypad bits.
    ldr r4, =0x04000130
    ldrh r2, [r4]
    mvn r2, r2
    lsl r2, r2, #22
    lsr r2, r2, #22

    @ Rising edges only.
    bic r3, r2, r8
    mov r8, r2

    cmp r3, #0
    beq wait_visible

    @ Right -> next state, modulo eight.
    tst r3, #16
    beq check_left

    add r5, r5, #1
    and r5, r5, #7
    b apply

check_left:
    @ Left -> previous state, modulo eight.
    tst r3, #32
    beq wait_visible

    subs r5, r5, #1
    movmi r5, #7
    b apply

    .ltorg
    .space 8
