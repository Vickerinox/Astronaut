
MEMORY
{
  EXPLOIT_MEM : ORIGIN = 0x02010000, LENGTH = 0x40000
}

/* The entry point */
ENTRY(_start);

SECTIONS
{
  .rodata :
  {
    *(.rodata .rodata.*);
  } > EXPLOIT_MEM

  .text :
  {
    *(.text .text.*);
  } > EXPLOIT_MEM

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}