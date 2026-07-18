/* Memory layout of ourself when title.tmd is loaded */
/* minimum exploit mem size: 0x13048 */
MEMORY
{
  EXPLOIT_MEM : ORIGIN = 0x037DF27C, LENGTH = 0x13048
  AUX_MEM : ORIGIN = 0x06880000, LENGTH = 0x10000
}

/* The entry point */
ENTRY(_start);

SECTIONS
{
  .text_aux : 
  {
    *(.text_aux);
  } > AUX_MEM


  .rodata_main :
  {
    *(.rodata .rodata.*);
  } > EXPLOIT_MEM

  .text_main :
  {
    *(.text .text.*);
  } > EXPLOIT_MEM



  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}