/* Memory layout of ourself when title.tmd is loaded */
/* minimum exploit mem size: 0x13048 */
MEMORY
{
  TMD_REGION1 : ORIGIN = 0x00208, LENGTH = 0x13048
  TMD_REGION2 : ORIGIN = 0x13800, LENGTH = 0x10000

  EXPLOIT_MEM : ORIGIN = 0x037DF278, LENGTH = 0x13048
  AUX_MEM : ORIGIN = 0x06880000, LENGTH = 0x10000
}

/* The entry point */
ENTRY(_start);

SECTIONS
{

  .rodata_main :
  {
    *(.rodata .rodata.*);
  } > EXPLOIT_MEM

  .text_main :
  {
    *(.text .text.*);
  } > EXPLOIT_MEM

  .data_main :
  {
    *(.data .data.*);
  } > EXPLOIT_MEM

  .bss_main :
  {
    *(.bss .bss.*);
  } > EXPLOIT_MEM

  .text_aux : 
  {
    *(.text_aux);
  } > AUX_MEM


  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}