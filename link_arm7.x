/* Memory layout of ourself once we've been taken over by the arm9 */
MEMORY
{
  SACRED_MEM_2 : ORIGIN = 0x037F0004, LENGTH = 0x6000
}

/* The entry point */
ENTRY(_start);

SECTIONS
{
  .rodata :
  {
    *(.rodata .rodata.*);
  } > SACRED_MEM_2

  .text :
  {
    *(.text .text.*);
  } > SACRED_MEM_2

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}