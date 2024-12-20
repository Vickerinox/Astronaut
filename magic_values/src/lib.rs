#![no_std]

pub const ARM_BRANCH_INSTRUCTION: u32 = 0xEA000000;
pub const ARM7_ENTRYPOINT_ADDRESS: usize = 0x37B8000 + (ARM7_ENTRYPOINT_OFFSET * 4);
pub const ARM7_BINARY_HEADER_SIZE: usize = 4;
pub const ARM9_MAGIC_ENTRYPOINT_ADDRESS: usize = 0x1329C;

//Whole word offsets into stolen arm7 WRAM
pub const ARM7_ENTRYPOINT_OFFSET: usize = 0x7FF; //cannot be within the jumptable, hopefully for obvious reasons.
pub const ARM7_JT_START: usize = 0x500;
pub const ARM7_JT_END: usize = 0x780;
pub const ARM7_JT_RANGE: core::ops::Range<usize> = ARM7_JT_START..ARM7_JT_END;

pub const fn create_arm_branch_instruction(jump_offset: i32) -> u32 {
    (0xFFFFFEu32.wrapping_add_signed(jump_offset) & 0xFFFFFF) | ARM_BRANCH_INSTRUCTION
}
