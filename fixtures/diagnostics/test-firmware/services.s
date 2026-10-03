.syntax unified
.cpu arm7tdmi
.arm
.section .text.entry,"ax",%progbits
.global _start
/* Original regression guest: successful upstream runs do not invoke their
 * failure-reporting SWIs. Exercise signed division and both exception returns. */
_start:
    mov r12, #1
    mov r0, #0
    msr cpsr_f, r0
    mrs r8, cpsr
    mvn r0, #122
    mov r1, #10
    swi 0x060000
    mrs r9, cpsr
    cmp r8, r9
    bne done
    cmn r0, #12
    bne done
    cmn r1, #3
    bne done
    cmp r3, #12
    bne done
    mov r12, #2
    adr r0, thumb_entry + 1
    bx r0
.thumb
.thumb_func
thumb_entry:
    movs r0, #123
    movs r1, #10
    swi #6
    cmp r0, #12
    bne thumb_failure
    cmp r1, #3
    bne thumb_failure
    cmp r3, #12
    bne thumb_failure
    movs r2, #0
    mov r12, r2
thumb_failure:
    adr r2, done
    bx r2
.arm
.balign 4
.org 0x400
done:
    b done
    .space 12
