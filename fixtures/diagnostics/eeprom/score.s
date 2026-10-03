.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry, "ax", %progbits
.global _start
_start:
    b entry
    .space 188
entry:
    ldr sp, =0x03007f00
    ldr r0, =0x04000000
    ldr r1, =0x0403
    strh r1, [r0]
    ldr r10, =0x0d000000
    ldr r11, =0x03000000
    mov r12, #1
    bl read_record
    ldr r2, =0x02000400
    ldrb r0, [r2]
    cmp r0, #0xa5
    ldrbeq r9, [r2, #1]
    movne r9, #0
    movne r8, #0
    moveq r8, #1
    cmp r9, #16
    movhi r9, #0
    cmp r8, #0
    bleq save_score
    mov r12, #0
    bl read_record
    ldr r2, =0x02000400
    ldrb r0, [r2, #1]
    strh r0, [r11, #4]
    mov r8, #0
    mov r0, #0x76
    strh r0, [r11]
    strh r9, [r11, #2]
    bl draw
frame:
    ldr r0, =0x04000006
leave_vblank:
    ldrh r1, [r0]
    cmp r1, #160
    bhs leave_vblank
wait_vblank:
    ldrh r1, [r0]
    cmp r1, #160
    blo wait_vblank
    ldr r0, =0x04000130
    ldrh r1, [r0]
    and r1, r1, #1
    cmp r1, #0
    bne released
    cmp r8, #0
    bne record
    cmp r9, #16
    addlo r9, r9, #1
    bl save_score
    mov r8, #1
    b record
released:
    mov r8, #0
record:
    strh r9, [r11, #2]
    bl draw
    b frame

failed:
    mov r0, #0xff
    strh r0, [r11]
    b failed

@ The guest owns score and rendering. Each point fills one eight-pixel cell;
@ cleared cells are black, so a restored score has an exact complete-frame image.
draw:
    push {r8}
    mov r7, lr
    ldr r2, =0x06000000
    mov r3, #8
    mov r6, r9, lsl #3
row:
    mov r4, #0
pixel:
    cmp r4, r6
    movlo r5, #0x03e0
    movhs r5, #0
    strh r5, [r2], #2
    add r4, r4, #1
    cmp r4, #128
    blo pixel
    add r2, r2, #224
    subs r3, r3, #1
    bne row
    @ Render first block's independently restored value in a second blue band.
    mov r12, #0
    bl read_record
    ldr r2, =0x02000400
    ldrb r0, [r2, #1]
    sub r8, r0, #16
    mov r6, r8, lsl #3
    ldr r2, =0x06000f00
    mov r3, #8
first_block_row:
    mov r4, #0
first_block_pixel:
    cmp r4, r6
    movlo r5, #0x7c00
    movhs r5, #0
    strh r5, [r2], #2
    add r4, r4, #1
    cmp r4, #128
    blo first_block_pixel
    add r2, r2, #224
    subs r3, r3, #1
    bne first_block_row
    pop {r8}
    bx r7
@ Persist distinct records in the first and last physical blocks. All guest
@ traffic uses DMA3 halfwords; no host operation supplies score or pixel values.
save_score:
    push {r0-r7, lr}
    ldr r2, =0x02000400
    mov r0, #0
    str r0, [r2]
    str r0, [r2, #4]
    mov r0, #0x5a
    strb r0, [r2]
    add r0, r9, #16
    strb r0, [r2, #1]
    strh r0, [r11, #4]
    mov r12, #0
    bl write_record
    mov r0, #0xa5
    strb r0, [r2]
    strb r9, [r2, #1]
    mov r12, #1
    bl write_record
    @ Read the committed record back through the chip before displaying success.
    bl read_record
    ldrb r0, [r2]
    cmp r0, #0xa5
    bne failed
    ldrb r0, [r2, #1]
    cmp r0, r9
    bne failed
    pop {r0-r7, lr}
    bx lr

@ Build MSB-first address clocks. The large device uses fourteen clocks with
@ only its low ten address bits selecting a block; r12 chooses first or last.
address_bits:
    .ifdef LARGE
    mov r4, #14
    mov r5, #1024
    sub r5, r5, #1
    .else
    mov r4, #6
    mov r5, #63
    .endif
    cmp r12, #0
    moveq r5, #0
address_loop:
    sub r4, r4, #1
    mov r6, r5, lsr r4
    and r6, r6, #1
    strh r6, [r3], #2
    cmp r4, #0
    bne address_loop
    bx lr

@ Write request: 10, address, 64 data bits, zero stop bit. An idle halfword
@ read polls ready after DMA completion, independently of the command response.
write_record:
    push {r0-r7, lr}
    ldr r3, =0x02000000
    mov r0, #1
    strh r0, [r3], #2
    mov r0, #0
    strh r0, [r3], #2
    bl address_bits
    ldr r2, =0x02000400
    mov r4, #0
data_byte:
    ldrb r5, [r2, r4]
    mov r6, #8
data_bit:
    sub r6, r6, #1
    mov r0, r5, lsr r6
    and r0, r0, #1
    strh r0, [r3], #2
    cmp r6, #0
    bne data_bit
    add r4, r4, #1
    cmp r4, #8
    blo data_byte
    mov r0, #0
    strh r0, [r3]
    ldr r0, =0x02000000
    .ifdef LARGE
    mov r1, #81
    .else
    mov r1, #73
    .endif
    mov r2, r10
    bl transfer
ready:
    ldrh r0, [r10]
    tst r0, #1
    beq ready
    pop {r0-r7, lr}
    bx lr

@ Read request: 11, address, zero stop bit. Decode the 68-clock response after
@ discarding its four dummy clocks; the record buffer is ordinary guest RAM.
read_record:
    push {r0-r7, lr}
    ldr r3, =0x02000000
    mov r0, #1
    strh r0, [r3], #2
    strh r0, [r3], #2
    bl address_bits
    mov r0, #0
    strh r0, [r3]
    ldr r0, =0x02000000
    .ifdef LARGE
    mov r1, #17
    .else
    mov r1, #9
    .endif
    mov r2, r10
    bl transfer
    mov r0, r10
    mov r1, #68
    ldr r2, =0x02000200
    bl transfer
    ldr r3, =0x02000208
    ldr r2, =0x02000400
    mov r4, #8
read_byte:
    mov r5, #0
    mov r6, #8
read_bit:
    ldrh r0, [r3], #2
    and r0, r0, #1
    orr r5, r0, r5, lsl #1
    subs r6, r6, #1
    bne read_bit
    strb r5, [r2], #1
    subs r4, r4, #1
    bne read_byte
    pop {r0-r7, lr}
    bx lr

@ DMA enable latches the descriptor. Polling waits until the hardware releases
@ bus ownership; incrementing addresses clock each halfword into the serial bus.
transfer:
    ldr r3, =0x040000d4
    str r0, [r3]
    str r2, [r3, #4]
    strh r1, [r3, #8]
    mov r0, #0x8000
    strh r0, [r3, #10]
dma_wait:
    ldrh r0, [r3, #10]
    tst r0, #0x8000
    bne dma_wait
    bx lr
    .ltorg
    .asciz "EEPROM_V124"
    .balign 4
