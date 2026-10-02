@ Original MIT window/mosaic diagnostic. All controls are applied during VBlank.
.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
_start:
 ldr r0, =0x04000000
 mov r1, #128
 strh r1, [r0]
 ldr r4, =0x07000000
 mov r1, #512
 mov r2, #128
clear_oam:
 strh r1, [r4], #8
 subs r2, r2, #1
 bne clear_oam
 @ BG0 is a striped foreground; BG1 is a solid green lower layer.
 ldr r4, =0x06000000
 ldr r1, =0x2121
 mov r2, #16
bg_tile:
 strh r1, [r4], #2
 tst r2, #1
 eorne r1, r1, #0x3300
 eorne r1, r1, #0x33
 subs r2, r2, #1
 bne bg_tile
 ldr r1, =0x3333
 mov r2, #16
bg_lower:
 strh r1, [r4], #2
 subs r2, r2, #1
 bne bg_lower
 ldr r4, =0x06009000
 mov r1, #1
 mov r2, #1024
map_lower:
 strh r1, [r4], #2
 subs r2, r2, #1
 bne map_lower
 ldr r4, =0x05000000
 mov r1, #0
 strh r1, [r4]
 mov r1, #31
 strh r1, [r4, #2]
 ldr r1, =0x7c00
 strh r1, [r4, #4]
 mov r1, #0x3e0
 strh r1, [r4, #6]
 ldr r4, =0x05000200
 ldr r1, =0x7fff
 strh r1, [r4, #2]
 @ Both objects share an alternating opaque/transparent source.
 ldr r4, =0x06010000
 ldr r1, =0x0101
 mov r2, #16
obj_tile:
 strh r1, [r4], #2
 tst r2, #1
 eorne r1, r1, #0x1100
 eorne r1, r1, #0x11
 subs r2, r2, #1
 bne obj_tile
 ldr r4, =0x07000000
 ldr r1, =0x0838
 strh r1, [r4]
 mov r1, #96
 strh r1, [r4, #2]
 mov r1, #0
 strh r1, [r4, #4]
 mov r1, #60
 strh r1, [r4, #8]
 mov r1, #100
 strh r1, [r4, #10]
 mov r1, #0
 strh r1, [r4, #12]
 mov r5, #0
 mov r6, #0
 mov r7, #0
 mov r9, #0                 @ Normal, wrapped or empty WIN0 bounds.
 mov r8, #0
apply:
 @ WIN0 wins overlap; WIN1 exposes BG1; object window exposes BG0 only.
 ldr r1, =0x2850
 add r1, r1, r5, lsl #8
 add r1, r1, r5
 cmp r9, #1
 ldreq r1, =0xdc14
 cmp r9, #2
 ldreq r1, =0x5050
 strh r1, [r0, #0x40]
 ldr r1, =0x466e
 strh r1, [r0, #0x42]
 ldr r1, =0x2850
 strh r1, [r0, #0x44]
 ldr r1, =0x3c64
 strh r1, [r0, #0x46]
 ldr r1, =0x0232
 cmp r6, #0
 ldreq r1, =0x0233
 strh r1, [r0, #0x48]
 ldr r1, =0x0112
 strh r1, [r0, #0x4a]
 ldr r1, =0x1000
 orr r1, r1, r7, lsl #6
 strh r1, [r0, #8]
 ldr r1, =0x1201
 strh r1, [r0, #10]
 ldr r1, =0x2233
 strh r1, [r0, #0x4c]
 ldr r4, =0x07000000
 mov r1, #60
 orr r1, r1, r7, lsl #12
 strh r1, [r4, #8]
 ldr r1, =0xf340
 strh r1, [r0]
 ldr r4, =0x03000000
 mov r1, #0xa4
 strh r1, [r4]
 strh r5, [r4, #2]
 strh r6, [r4, #4]
 strh r7, [r4, #6]
 strh r9, [r4, #8]
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
 bic r3, r2, r8
 mov r8, r2
 cmp r3, #0
 beq wait_visible
 tst r3, #16
 addne r5, r5, #16
 and r5, r5, #0x70
 tst r3, #32
 subne r5, r5, #16
 and r5, r5, #0x70
 tst r3, #1
 eorne r6, r6, #1
 tst r3, #2
 eorne r7, r7, #1
 tst r3, #64
 movne r9, #1
 tst r3, #128
 movne r9, #2
 b apply
 .ltorg
 .space 8
