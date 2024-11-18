#![no_main]
#![no_std]

mod allocator;
const FONT_FILE: &[u8] = include_bytes!("./font.bin");

pub struct TextLayoutPass {
    current_position: (u8, u8)
}
impl TextLayoutPass {
    pub fn new() -> Self {
        Self { current_position: (0,0)}
    }
    pub fn layout_str(&mut self, str: &str) {
        for char in str.chars() {
            self.layout_char(char);
        }
    }
    pub fn layout_char(&mut self, char: core::primitive::char) {
        const CHAR_WIDTH: u32 = 7 << 4; //(i.e, 1*7 texels)
        let index = match char.is_ascii() {
            true => char as u8,
            false => '?' as u8,
        };
        let index = CHAR_WIDTH * index as u32;
        let x = self.current_position.0 as u16;
        let y = self.current_position.1 as u16;
        unsafe {
            core::ptr::write_volatile(0x4000488 as *mut u32, index | (8 << 20)); //set UV coordinates
            core::ptr::write_volatile(0x4000490 as *mut u32, (x | (y<<10)) as u32); //draw vertex
    
            core::ptr::write_volatile(0x4000488 as *mut u32, (index+CHAR_WIDTH) | (8 << 20)); //set UV coordinates
            core::ptr::write_volatile(0x40004A0 as *mut u32, 224); //draw vertex
    
            core::ptr::write_volatile(0x4000488 as *mut u32, index+CHAR_WIDTH); //set UV coordinates
            core::ptr::write_volatile(0x40004A0 as *mut u32, 320 << 10); //draw vertex
    
            core::ptr::write_volatile(0x4000488 as *mut u32, index); //set UV coordinates
            core::ptr::write_volatile(0x40004A0 as *mut u32, 0b1111111111-223); //draw vertex
        }
        self.current_position.0 += 4;
    }
}
extern crate alloc;
use alloc::string::String;
/// This function steals control of the ARM7 CPU assuming it is running in the sync loop within the bootloader.
/// The way it does this is by stealing some unused WRAM, writing a jump table to "stabilize" it,
/// binary at the destination of the jump table, and mapping it to the memory where the ARM7 is executing
pub unsafe fn steal_arm7() {
    //offsets in words (4 bytes) into the WRAM were about to steal
    //JT here means "jump table", offsets could likely be tweaked, but i've not bothered to tune them.
    const ENTRYPOINT_OFFSET: usize = 0x800; //cannot be within the jumptable, hopefully for obvious reasons.
    const JT_START: usize = 0x500;
    const JT_END: usize = 0x780;
    const BRANCH_BASE: usize = ENTRYPOINT_OFFSET - 2; //because ARM instructions are "special", this is correct.

    //some magic constants never hurt ;)
    const BLANK_BRANCH_INSTRUCTION: u32 = 0xEA000000;
    const STOLEN_WRAM: *mut u32 = 0x03000000 as *mut u32;

    //steal WRAM-C4 from the arm7, as it *should* be unused. It is owned by the arm9 in slot0
    core::ptr::write_volatile(0x4004050 as *mut u8, 0b10000000);
    //map WRAM-C4 to 0x0300_0000..0x0300_8000 on our side. Which should also be unused right now.
    core::ptr::write_volatile(0x400405C as *mut u32, 1 << 19);

    //Write our jump table to the WRAM
    for i in JT_START..JT_END {
        core::ptr::write_volatile(
            STOLEN_WRAM.add(i),
            BLANK_BRANCH_INSTRUCTION | ((BRANCH_BASE - i) & 0xFFFFFF) as u32,
        );
    }
    //Write our entrypoint (for now an infinte loop instruction)
    core::ptr::write_volatile(STOLEN_WRAM.add(ENTRYPOINT_OFFSET), 0xEAFFFFFE);

    //overwrite the WRAM bank the arm7 is currently executing in with ours
    //Set WRAM-C4 to slot 7 on arm7, replacing the bank it's currently executing in. (WRAM-C7)
    core::ptr::write_volatile((0x4004050) as *mut u8, 0b10011101);
    //disable the old WRAM bank (WRAM-C7) entirely (unneccesary maybe?)
    core::ptr::write_volatile((0x4004053) as *mut u8, 0);

    //congrats! Now the arm7 is stolen. Since it will immediately jump to it's entrypoint.
}
pub unsafe fn steal_main_mem() {
    allocator::ALLOCATOR.init();
}

/// Main
#[no_mangle]
pub fn _start() {
    unsafe {
        //enable the 2D engine A, with no backgrounds on.
        core::ptr::write_volatile(0x4000000 as *mut u32, 0b000000000000000010000000000000000);
        core::ptr::write_volatile(0x4001000 as *mut u32, 0b000000000000000010000000000000000);
        
        //set background color to brat green.
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b1111100000111111);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b1111100000111111);

        steal_arm7();
        //steal_main_mem();

        //let mut lmao = String::from("kachow!!!");
        
        
        core::ptr::write_volatile(0x4000304 as *mut u16, 12); 
        core::ptr::write_volatile(0x04000240 as *mut u8, 0x80); //enable VRAM bank A
        core::ptr::write_volatile(0x04000244 as *mut u8, 0x80); //enable VRAM bank E

        //write to "color palette 0"
        core::ptr::write_volatile(0x06880000 as *mut u16, 0b0_00000_00000_00000);
        core::ptr::write_volatile(0x06880004 as *mut u16, 0b0_11111_00000_00000);
        core::ptr::write_volatile(0x06880002 as *mut u16, 0b0_11111_11111_11111);
        core::ptr::write_volatile(0x06880006 as *mut u16, 0b0_00000_00000_11111);

        //copy font to vram
        for (i, w) in FONT_FILE.chunks_exact(4).enumerate() {
            let reg = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            core::ptr::write_volatile((0x6800000 as *mut u32).add(i), reg);
        }

        //setup 3d hardware
        use reboot_lib::{VIDEO_HARDWARE, PrimaryDisplayControl, MatrixMode, Viewport, VertexListType};
        VIDEO_HARDWARE.vram_control_bank_a.write(0x83); //map VRAM BANK A
        VIDEO_HARDWARE.vram_control_bank_e.write(0x83); //map VRAM BANK E
        VIDEO_HARDWARE.primary_display_control.write(PrimaryDisplayControl::BG_MODE_0 | PrimaryDisplayControl::ENABLE_3D | PrimaryDisplayControl::ENABLE_BG_0);
        VIDEO_HARDWARE.display_control_3d.write(1); //enables texture mapping
        VIDEO_HARDWARE.geometry_commands.pipeline_swap_buffers.write(0); //swap geometry buffers
        
        //init matricies
        VIDEO_HARDWARE.geometry_commands.matrix_mode.write(MatrixMode::PROJECTION);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack
        VIDEO_HARDWARE.geometry_commands.matrix_mode.write(MatrixMode::POSITION);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack
        VIDEO_HARDWARE.geometry_commands.matrix_mode.write(MatrixMode::TEXTURE);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack
        VIDEO_HARDWARE.geometry_commands.matrix_mode.write(MatrixMode::VECTOR);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack
        //VIDEO_HARDWARE.geometry_commands.matrix_mode.write(MatrixMode::PROJECTION);
        //VIDEO_HARDWARE.geometry_commands.matrix_mult_scale.write(0x1000000);
        //VIDEO_HARDWARE.geometry_commands.matrix_mult_scale.write(0x1000000);
        //VIDEO_HARDWARE.geometry_commands.matrix_mult_scale.write(0x1000000);
        
        //more init
        VIDEO_HARDWARE.geometry_commands.pipeline_set_viewport.write(Viewport::WHOLE_SCREEN_DEFAULT);
        VIDEO_HARDWARE.geometry_commands.material_texture_attributes.write((7 << 20) | (2 <<26) | (1<<29)); //bind font texture
        VIDEO_HARDWARE.geometry_commands.material_color_palette.write(0); //use color palette 0
        VIDEO_HARDWARE.geometry_commands.material_polygon_attributes.write((1 << 6) | (1 << 7) | (31 << 16)); //use max alpha, and no culling
        VIDEO_HARDWARE.clear_depth.write(0x7FFF); //max depth
        VIDEO_HARDWARE.clear_color.write(0b0000111101010100); //greenish color

        //draw stuff
        VIDEO_HARDWARE.geometry_commands.pipeline_begin_vertex_list.write(VertexListType::IndividualQuads);
        VIDEO_HARDWARE.geometry_commands.vertex_set_color.write(0x7FFF); //white
        TextLayoutPass::new().layout_str("Hello World! Bänner");
        VIDEO_HARDWARE.geometry_commands.pipeline_end_vertex_list.write(0);
        VIDEO_HARDWARE.geometry_commands.pipeline_swap_buffers.write(0);

        //core::ptr::write_volatile(0x5000000 as *mut u16, 0b0000111101010100);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0000111101010100);
        //core::ptr::write_volatile(0x5000000 as *mut u16, 0b1111100000000001);
        
        loop {}
    }
}
//Really our code should NEVER panic, but we still need this.
#[cfg(not(test))] //works to shut up rust-analyzer in vscode. It keeps thinking we still have std...
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        //enable the 2D engine A, with no backgrounds on.
        core::ptr::write_volatile(0x4000000 as *mut u32, 0b000000000000000010000000000000000);
        //set background color to brat green.
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b1111100000000001);
    }
    loop {}
}
