OUTPUT_FORMAT("elf32-littlearm")
OUTPUT_ARCH(arm)
ENTRY(_start)
SECTIONS {
    . = 0;
    .text : { KEEP(*(.text.entry)) *(.text*) *(.rodata*) }
    /DISCARD/ : { *(.comment*) *(.note*) *(.ARM.attributes*) }
}
