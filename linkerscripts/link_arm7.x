/* Memory layout of ourself once we've been taken over by the arm9 */
MEMORY
{
  BIN_SPACE : ORIGIN = 0x0, LENGTH = 0x8000
  SACRED_MEM_2 : ORIGIN = 0x06000000, LENGTH = 0x8000
}

/* The entry point */
ENTRY(_start);

SECTIONS
{
  .text_entry : 
  {
    *(.text_entry);
  } > SACRED_MEM_2 AT > BIN_SPACE

  .text :
  {
    *(.text .text.*);
  } > SACRED_MEM_2 AT > BIN_SPACE

  .rodata :
  {
    *(.rodata .rodata.*);
  } > SACRED_MEM_2 AT > BIN_SPACE

  .data :
  {
    *(.data .data.*);
  } > SACRED_MEM_2 AT > BIN_SPACE

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}