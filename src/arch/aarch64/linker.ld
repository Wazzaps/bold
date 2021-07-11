SECTIONS {
  . = 0x80000; /* Raspbery Pi 3 Aarch64 (kernel8.img) load address */

  /* start of the binary */
  _start = .;

  .text : {
      KEEP(*(.text.init)) /* from init.S */
      *(.text .text.* .gnu.linkonce.t*)
  }

  . = ALIGN(4096);
  __data_start = .;

  .rodata : {
    *(.rodata .rodata.* .gnu.linkonce.r*)
    . = ALIGN(8);
    __drivers_start = .;
    KEEP(*(.drivers*))
    __drivers_end = .;
  }

  .data : {
    *(.data .data.* .gnu.linkonce.d* .got .got.plt)
  }

  . = ALIGN(4096);
  __dma_start = .;
  .dma : {
    KEEP(*(.dma))
  }
  . = ALIGN(4096);
  __dma_end = .;

  .bss (NOLOAD) : {
    . = ALIGN(32);
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(8);
    __bss_end = .;
  }

  /* end of the binary */
  _end = ALIGN(8);
  __ram_start = ALIGN(4096);
  . = 0x40000000;
  __ram_end = .;

  /* number of bytes in BSS section and complete binary */
  __bss_length = (__bss_end - __bss_start);
  __binary_length = (_end - _start);

  /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}