/* Memory layout of ourself when title.tmd is loaded */
/* minimum exploit mem size: 0x13048 */
MEMORY
{
  EXPLOIT_MEM : ORIGIN = 0x037DF27C, LENGTH = 0x13048
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