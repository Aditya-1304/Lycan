@ Original MIT Slice-25 raster/HBlank-DMA diagnostic.
@
@ State 0:
@   DMA0 writes an increasing brightness value once per visible HBlank.
@
@ State 1:
@   DMA3 writes a decreasing brightness value.
@
@ State 2:
@   DMA0 and DMA3 request simultaneously. DMA0 must complete first because it
@   has higher priority, then DMA3 overwrites the same effect registers. The
@   resulting image therefore matches state 1 while IF proves both channels ran.
@
@ Each transfer writes:
@   BLDALPHA = channel sentinel
@   BLDY     = brightness for the NEXT visible scanline
@
@ This directly checks that the current scanline was drawn before the HBlank
@ register write and the following scanline observes the new value.

.syntax unified
.cpu arm7tdmi
.arm

.section .text.entry, "ax", %progbits
.global _start

_start:
    ldr r0, =0x04000000

    @ Forced blank while constructing the scene.
    mov r1, #0x80
    strh r1, [r0]

    @ ------------------------------------------------------------
    @ BG0: one solid 4bpp tile, repeated over the whole screen.
    @ ------------------------------------------------------------
    ldr r4, =0x06000000
    ldr r1, =0x11111111
    mov r2, #8

fill_tile:
    str r1, [r4], #4
    subs r2, r2, #1
    bne fill_tile

    @ Screen block 16 -> tile 0 everywhere.
    ldr r4, =0x06008000
    mov r1, #0
    mov r2, #1024

fill_map:
    strh r1, [r4], #2
    subs r2, r2, #1
    bne fill_map

    @ Palette index 1 = mid-level RGB555 color.
    ldr r4, =0x05000000
    ldr r1, =0x4210
    strh r1, [r4, #2]

    @ BG0 priority 0, character base 0, screen block 16.
    ldr r1, =0x1000
    strh r1, [r0, #8]

    @ Brightness-increase mode, BG0 as first target.
    mov r1, #0x81
    strh r1, [r0, #0x50]

    @ ------------------------------------------------------------
    @ Copy the immutable ROM raster tables into IWRAM.
    @ DMA0 cannot source cartridge ROM, so both channels use internal RAM.
    @ ------------------------------------------------------------
    ldr r6, =dma0_table_rom
    ldr r7, =0x03001000
    mov r8, #160

copy_dma0_table:
    ldr r1, [r6], #4
    str r1, [r7], #4
    subs r8, r8, #1
    bne copy_dma0_table

    ldr r6, =dma3_table_rom
    ldr r7, =0x03001400
    mov r8, #160

copy_dma3_table:
    ldr r1, [r6], #4
    str r1, [r7], #4
    subs r8, r8, #1
    bne copy_dma3_table

    @ ------------------------------------------------------------
    @ Diagnostic mailbox.
    @ +0 = ID 0x00a6
    @ +2 = state
    @ +4 = DMA completion IF snapshot from preceding visible field
    @ +6 = number of VBlank re-arms
    @ ------------------------------------------------------------
    ldr r11, =0x03000000

    mov r1, #0xa6
    strh r1, [r11]

    mov r5, #0
    strh r5, [r11, #2]

    mov r1, #0
    strh r1, [r11, #4]
    strh r1, [r11, #6]

    @ Previous active-low keypad sample, converted to pressed bits.
    mov r9, #0

    @ Clear any stale DMA0/DMA3 completion flags.
    ldr r4, =0x04000202
    ldr r1, =0x0900
    strh r1, [r4]

    b configure

@ -----------------------------------------------------------------
@ Re-arm selected HBlank DMA descriptors once per VBlank.
@ -----------------------------------------------------------------
configure:
    @ Disable first so the next enable edge reloads source/destination/count.
    mov r1, #0
    strh r1, [r0, #0xba]
    strh r1, [r0, #0xde]

    @ Line 0 is initialized explicitly because no previous visible HBlank
    @ exists to populate its BLDY value.
    cmp r5, #0
    bne initial_reverse

initial_forward:
    ldr r1, =0x0404
    strh r1, [r0, #0x52]

    mov r1, #0
    strh r1, [r0, #0x54]
    b configure_channels

initial_reverse:
    ldr r1, =0x0c0c
    strh r1, [r0, #0x52]

    mov r1, #15
    strh r1, [r0, #0x54]

configure_channels:
    cmp r5, #0
    beq configure_dma0_only

    cmp r5, #1
    beq configure_dma3_only

    @ ------------------------------------------------------------
    @ State 2: arm DMA3 first on purpose, then DMA0.
    @ Hardware priority must still be DMA0 -> DMA3.
    @ ------------------------------------------------------------
configure_both:
    ldr r4, =0x040000d4
    ldr r1, =0x03001400
    str r1, [r4]

    ldr r1, =0x04000052
    str r1, [r4, #4]

    @ Count=2 halfwords.
    @ Enable + IRQ + HBlank + repeat + destination increment/reload.
    ldr r1, =0xe2600002
    str r1, [r4, #8]

    ldr r4, =0x040000b0
    ldr r1, =0x03001000
    str r1, [r4]

    ldr r1, =0x04000052
    str r1, [r4, #4]

    ldr r1, =0xe2600002
    str r1, [r4, #8]

    b enable_display

configure_dma0_only:
    ldr r4, =0x040000b0

    ldr r1, =0x03001000
    str r1, [r4]

    ldr r1, =0x04000052
    str r1, [r4, #4]

    ldr r1, =0xe2600002
    str r1, [r4, #8]

    b enable_display

configure_dma3_only:
    ldr r4, =0x040000d4

    ldr r1, =0x03001400
    str r1, [r4]

    ldr r1, =0x04000052
    str r1, [r4, #4]

    ldr r1, =0xe2600002
    str r1, [r4, #8]

enable_display:
    @ Mode 0, BG0.
    mov r1, #0x100
    strh r1, [r0]

@ -----------------------------------------------------------------
@ Wait for one complete visible field, snapshot DMA completion flags,
@ process one input edge, then re-arm for the next field.
@ -----------------------------------------------------------------
wait_visible:
    ldrh r1, [r0, #6]
    cmp r1, #160
    bhs wait_visible

wait_vblank:
    ldrh r1, [r0, #6]
    cmp r1, #160
    blo wait_vblank

    @ Both completion IRQ bits latch even though IE/IME are disabled.
    ldr r4, =0x04000202
    ldrh r1, [r4]

    ldr r3, =0x0900
    and r2, r1, r3
    strh r2, [r11, #4]

    @ W1C DMA0 + DMA3 flags before the next field.
    strh r3, [r4]

    @ Active-low KEYINPUT -> pressed-bit representation.
    ldr r4, =0x04000130
    ldrh r2, [r4]
    mvn r2, r2
    lsl r2, r2, #22
    lsr r2, r2, #22

    @ Rising edges only.
    bic r3, r2, r9
    mov r9, r2

    @ Right cycles 0 -> 1 -> 2 -> 0.
    tst r3, #16
    beq check_left

    add r5, r5, #1
    cmp r5, #3
    moveq r5, #0
    b input_done

check_left:
    @ Left cycles 0 -> 2 -> 1 -> 0.
    tst r3, #32
    beq input_done

    subs r5, r5, #1
    movmi r5, #2

input_done:
    strh r5, [r11, #2]

    ldrh r1, [r11, #6]
    add r1, r1, #1
    strh r1, [r11, #6]

    b configure

    .ltorg

.section .rodata, "a", %progbits
.balign 4

dma0_table_rom:
    .incbin "dma0-table.bin"

dma3_table_rom:
    .incbin "dma3-table.bin"

    .space 8
