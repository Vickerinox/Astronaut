
MEMORY
{
  EXPLOIT_MEM : ORIGIN = 0x02300000, LENGTH = 0x8000
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