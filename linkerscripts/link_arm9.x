/* Memory layout of ourself when title.tmd is loaded */
/* minimum exploit mem size: 0x13048 */
MEMORY
{
  TMD_REGION1 : ORIGIN = 0x00208, LENGTH = 0x13048
  TMD_REGION2 : ORIGIN = 0x13A08, LENGTH = 0x10000

  EXPLOIT_MEM : ORIGIN = 0x037DF278, LENGTH = 0x13048
  AUX_MEM : ORIGIN = 0x06880004, LENGTH = 0x10000
  ITCM : ORIGIN = 0x1000000, LENGTH = 0x7000
}

/* The entry point */
ENTRY(_start);

SECTIONS
{

  .rodata_main :
  {
    *(.rodata .rodata.*);
  } > EXPLOIT_MEM AT > TMD_REGION1

  .text_main :
  {
    *(.text .text.*);
  } > EXPLOIT_MEM AT > TMD_REGION1

  .data_main :
  {
    *(.data .data.*);
  } > EXPLOIT_MEM AT > TMD_REGION1
  
  .bss_main :
  {
    *(.bss .bss.*);
  } > EXPLOIT_MEM AT > TMD_REGION1

  itcm_start = .;
  .text_itcm : {
    *(.text .text.*);
  } > ITCM AT > TMD_REGION1
  itcm_end = .;

  aux_start = .;
  .text_aux : 
  {
    *(.text_aux);
  } > AUX_MEM AT > TMD_REGION2
  aux_end = .;





  PROVIDE(_aux_off = LOADADDR(.text_aux));
  PROVIDE(_aux_len = aux_end - aux_start);
  PROVIDE(_itcm_addr = itcm_start);
  PROVIDE(_itcm_len = itcm_end - itcm_start);

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}