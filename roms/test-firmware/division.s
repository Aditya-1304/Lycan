.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
/* Original test-only firmware. Only SWI 0x06 is supported; every other
 * vector/service traps instead of pretending to provide a retail BIOS. */
_start:
    b trap
    b trap
    b division
    b trap
    b trap
    b trap
    b trap
    b trap
trap:
    b trap
/* SWI preserves the caller's CPSR in SPSR_svc. ARM and Thumb encode the
 * service byte at LR-2; the CPU supplies the exception return address. */
division:
    stmdb sp!, {r4-r6}
    ldrb r2, [lr, #-2]
    cmp r2, #6
    bne trap
    cmp r1, #0
    beq trap
    mov r4, r0
    eor r6, r0, r1
    cmp r0, #0
    rsblt r0, r0, #0
    cmp r1, #0
    rsblt r1, r1, #0
    mov r5, r1
    mov r1, #0
    mov r3, #0
    mov r2, #32
/* Restoring division consumes one numerator bit per iteration. Carry
 * retains the 33rd remainder bit when doubling a large remainder. */
bit:
    movs r0, r0, lsl #1
    adcs r1, r1, r1
    cmpcc r1, r5
    subcs r1, r1, r5
    adc r3, r3, r3
    subs r2, r2, #1
    bne bit
    mov r0, r3
    cmp r6, #0
    rsblt r0, r0, #0
    cmp r4, #0
    rsblt r1, r1, #0
    ldmia sp!, {r4-r6}
    movs pc, lr
/* The CPU fetches ahead of the final return; map the complete BIOS window. */
.org 0x4000
