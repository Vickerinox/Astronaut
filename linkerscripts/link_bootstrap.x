/* Imaginary memory layout of the bootstap */
MEMORY
{
  BOOTSTRAP_MEM : ORIGIN = 0, LENGTH = 0x200
}

/* The entry point */
ENTRY(_start);

SECTIONS
{
  .rodata :
  {
    *(.rodata .rodata.*);
  } > BOOTSTRAP_MEM

  .text :
  {
    *(.text .text.*);
  } > BOOTSTRAP_MEM

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}