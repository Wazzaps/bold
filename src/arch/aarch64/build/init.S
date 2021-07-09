.section .text.init

.global _start

_start:
    // read cpu affinity, start core 0, halt rest
    mrs     x1, mpidr_el1
    and     x1, x1, #3
    cbz     x1, 2f

1:
    // core affinity != 0, halt it
    wfe
    b       1b

2:
    // set the stack to start before our boot code
    ldr     x1, =_start

    // set up EL1
    mrs     x3, CurrentEL
    and     x3, x3, #12       // clear reserved bits

    // running at EL3?
    cmp     x3, #12
    bne     5f
    // should never be executed, just for completeness
    mov     x2, #0x5b1
    msr     scr_el3, x2
    mov     x2, #0x3c9
    msr     spsr_el3, x2
    adr     x2, 5f
    msr     elr_el3, x2
    eret

5:
    // running at EL2?
    cmp     x3, #4
    beq     5f
    msr     sp_el1, x1
    // enable CNTP for EL1
    mrs     x3, cnthctl_el2
    orr     x3, x3, #3
    msr     cnthctl_el2, x3
    msr     cntvoff_el2, xzr
    // enable AArch64 in EL1
    mov     x3, #(1 << 31)    // AArch64
    orr     x3, x3, #(1 << 1)   // SWIO hardwired on Pi3
    msr     hcr_el2, x3
    mrs     x3, hcr_el2
    // Setup SCTLR access
    mov     x2, #0x0800
    movk    x2, #0x30d0, lsl #16
    msr     sctlr_el1, x2
    // change execution level to EL1
    mov     x2, #0x3c4
    msr     spsr_el2, x2
    adr     x2, 5f
    msr     elr_el2, x2
    eret

5:
    mov     sp, x1

    // load the start address and number of bytes in BSS section
    ldr     x1, =__bss_start
    ldr     x2, =__bss_length

3:
    // zero out the BSS section, 64-bits at a time
    cbz     x2, 4f
    str     xzr, [x1], #8
    sub     x2, x2, #8
    cbnz    x2, 3b

4:
    // jump to kmain, which shouldn't return. halt if it does
    bl      kmain
    b       1b