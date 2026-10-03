@ Original cartridge-clock diagnostic. SRAM records initialization independently
@ of RTC metadata; reset/reopen must read the configured clock without rewriting it.
.syntax unified
.cpu arm7tdmi
.arm

@ S-3511 command IDs from the SIIRTC_V001 wire protocol. Keep status and
@ seven-byte date/time transactions distinct; response data is LSB-first.
.equ STATUS_WRITE, 0x62
.equ STATUS_READ,  0x63
.equ DATE_WRITE,   0x64
.equ DATE_READ,    0x65
.section .text.entry, "ax", %progbits
.global _start
_start:
    b start
    .space 0xfc
start:
    ldr sp, =0x03007f00
    ldr r8, =0x080000c4
    mov r0, #1
    strh r0, [r8, #4]
    ldr r0, =0x04000000
    ldr r1, =0x0403
    strh r1, [r0]
    ldr r0, =0x0e000000
    ldrb r1, [r0]
    cmp r1, #1
    blne configure
poll:
    ldr r0, =0x04000130
    ldrh r1, [r0]
    tst r1, #1
    bleq configure
    ldr r0, =0x04000130
    ldrh r1, [r0]
    tst r1, #2
    bne read_clock
    mov r0, #STATUS_WRITE
    bl command
    mov r0, #0
    bl send
    mov r0, #0
    strh r0, [r8]
read_clock:
    mov r0, #DATE_READ
    bl command
    mov r0, #5
    strh r0, [r8, #2]
    ldr r4, =0x02000000
    mov r5, #7
read_date:
    bl receive
    strb r0, [r4], #1
    subs r5, r5, #1
    bne read_date
    mov r0, #0
    strh r0, [r8]
    mov r0, #STATUS_READ
    bl command
    mov r0, #5
    strh r0, [r8, #2]
    bl receive
    strb r0, [r4]
    mov r0, #0
    strh r0, [r8]
    ldr r0, =0x02000010
    mov r1, #0x30
    strh r1, [r0]
    bl display
    b poll
configure:
    push {r4-r7, lr}
    mov r0, #STATUS_WRITE
    bl command
    mov r0, #0x40
    bl send
    mov r0, #0
    strh r0, [r8]
    mov r0, #DATE_WRITE
    bl command
    ldr r4, =date
    mov r5, #7
write_date:
    ldrb r0, [r4], #1
    bl send
    subs r5, r5, #1
    bne write_date
    mov r0, #0
    strh r0, [r8]
    ldr r0, =0x0e000000
    mov r1, #1
    strb r1, [r0]
    pop {r4-r7, pc}
command:
    push {lr}
    mov r1, #7
    strh r1, [r8, #2]
    mov r1, #1
    strh r1, [r8]
    mov r1, #5
    strh r1, [r8]
    bl send
    pop {pc}
send:
    mov r2, #8
send_bit:
    and r1, r0, #1
    mov r1, r1, lsl #1
    orr r1, r1, #4
    strh r1, [r8]
    orr r1, r1, #1
    strh r1, [r8]
    mov r0, r0, lsr #1
    subs r2, r2, #1
    bne send_bit
    bx lr
receive:
    mov r0, #0
    mov r2, #0
receive_bit:
    mov r1, #4
    strh r1, [r8]
    mov r1, #5
    strh r1, [r8]
    ldrh r1, [r8]
    and r1, r1, #2
    mov r1, r1, lsr #1
    orr r0, r0, r1, lsl r2
    add r2, r2, #1
    cmp r2, #8
    bne receive_bit
    bx lr
@ Render eight BCD bytes as pairs of 3x5 digits, enlarged fourfold. Rows are
@ year/month/day/weekday/hour/minute/second/control, with black background.
display:
    push {r4-r11, lr}
    ldr r4, =0x02000000
    ldr r5, =0x06000000
    mov r6, #8
row:
    ldrb r7, [r4], #1
    mov r9, #2
nibble:
    mov r0, r7, lsr #4
    and r0, r0, #15
    ldr r1, =font
    add r1, r1, r0, lsl #1
    ldrh r10, [r1]
    mov r11, #20
pixel_y:
    mov r3, #12
    mov r2, #0
pixel_x:
    mov r0, r11
    rsb r0, r0, #20
    mov r0, r0, lsr #2
    add r0, r0, r0, lsl #1
    add r0, r0, r2, lsr #3
    mov r1, r10, lsr r0
    tst r1, #1
    mov r1, #0
    ldrne r1, =0x03e0
    strh r1, [r5, r2]
    add r2, r2, #2
    subs r3, r3, #1
    bne pixel_x
    add r5, r5, #480
    subs r11, r11, #1
    bne pixel_y
    sub r5, r5, #9600
    add r5, r5, #32
    mov r7, r7, lsl #4
    subs r9, r9, #1
    bne nibble
    sub r5, r5, #64
    add r5, r5, #9600
    subs r6, r6, #1
    bne row
    pop {r4-r11, pc}
.align 2
date: .byte 0x24, 0x02, 0x29, 4, 0x23, 0x59, 0x58
.align 2
font:
    .hword 0x7b6f,0x749a,0x73e7,0x79e7,0x49ed,0x79cf,0x7bcf,0x4927,0x7bef,0x79ef,0,0,0,0,0,0
    .asciz "SIIRTC_V001"
    .asciz "SRAM_V113"
