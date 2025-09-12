/* Memory layout of ourself once we've been taken over by the arm9 */
MEMORY
{
  SACRED_MEM_2 : ORIGIN = 0x0600000C, LENGTH = 0x4000
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