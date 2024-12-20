#![no_main]
#![no_std]

const FONT_FILE: &[u8] = include_bytes!("./font.bin");
const ARM7_BINARY: &[u8] = include_bytes!("./arm7.bin");
const TEST_STRING_UPPER: &str = "THE QUICK BROWN FOX JUMPED OVER THE LAZY DOG";
const TEST_STRING_LOWER: &str = "the quick brown fox jumped over the lazy dog";

use core::arch::asm;

use reboot_lib::{
    MatrixMode, PolygonAttributes, PrimaryDisplayControl, VideoPowerControl, Viewport,
    VIDEO_HARDWARE,
};
extern crate alloc;

mod mbr;


pub unsafe fn nocash_write(str: &str) {
    const NOCASH_OUT_CHR: *mut u8 = 0x4fffa1c as *mut u8;
    for byte in str.as_bytes() {
        NOCASH_OUT_CHR.write_volatile(*byte);
    }
}
/// This function steals control of the ARM7 CPU assuming it is running in the sync loop within the bootloader.
/// The way it does this is by stealing some unused WRAM, writing a jump table to "stabilize" it,
/// binary at the destination of the jump table, and mapping it to the memory where the ARM7 is executing
pub unsafe fn steal_arm7() {
    //offsets in words (4 bytes) into the WRAM were about to steal
    //JT here means "jump table", offsets could likely be tweaked, but i've not bothered to tune them.
    const ENTRYPOINT_OFFSET: usize = 0x7FF; //cannot be within the jumptable, hopefully for obvious reasons.
    const JT_START: usize = 0x500;
    const JT_END: usize = 0x780;
    const BRANCH_BASE: usize = ENTRYPOINT_OFFSET - 2; //because ARM instructions are "special", this is correct.

    //some magic constants never hurt ;)
    const BLANK_BRANCH_INSTRUCTION: u32 = 0xEA000000;
    const STOLEN_WRAM: *mut u32 = 0x03000000 as *mut u32;
    const BINARY_ENTRY_ADDR: *mut u32 =
        (0x03000000 + (ENTRYPOINT_OFFSET * size_of::<u32>())) as *mut u32;

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
    //Write our binary (for now an infinte loop instruction)
    for (j, c) in ARM7_BINARY.chunks_exact(4).enumerate() {
        let stuff = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        core::ptr::write_volatile(BINARY_ENTRY_ADDR.add(j), stuff);
    }

    //overwrite the WRAM bank the arm7 is currently executing in with ours
    //Set WRAM-C4 to slot 7 on arm7, replacing the bank it's currently executing in. (WRAM-C7)
    core::ptr::write_volatile((0x4004050) as *mut u8, 0b10011101);
    //disable the old WRAM bank (WRAM-C7) entirely (unneccesary maybe?)
    core::ptr::write_volatile((0x4004053) as *mut u8, 0);

    //congrats! Now the arm7 is stolen. Since it will immediately jump to it's entrypoint.
}
pub unsafe fn steal_main_mem() {
    reboot_lib::ALLOCATOR.init();
}
unsafe fn main() {
    unsafe {

        //enable the 2D engine A, with no backgrounds on.
        core::ptr::write_volatile(0x4000000 as *mut u32, 0b000000000000000010000000000000000);
        core::ptr::write_volatile(0x4001000 as *mut u32, 0b000000000000000010000000000000000);

        //set background color to brat green.
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b1111100000111111);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b1111100000111111);

        core::ptr::write_volatile(0x200_0000 as *mut u32, 0);
        
        reboot_lib::IPC_FIFO_HARDWARE.enable();
        reboot_lib::IPC_FIFO_HARDWARE.set_status(0);
        steal_arm7();

        core::ptr::write_volatile(0x4000304 as *mut u16, 12);
        core::ptr::write_volatile(0x04000240 as *mut u8, 0x80); //enable VRAM bank A
        core::ptr::write_volatile(0x04000244 as *mut u8, 0x80); //enable VRAM bank E

        //write to "color palette 0"
        core::ptr::write_volatile(0x06880000 as *mut u16, 0b0_00000_00000_00000);
        core::ptr::write_volatile(0x06880004 as *mut u16, 0b0_00000_00000_00000);
        core::ptr::write_volatile(0x06880002 as *mut u16, 0b0_11111_11111_11111);
        core::ptr::write_volatile(0x06880006 as *mut u16, 0b0_00000_00000_11111);

        //copy font to vram
        for (i, w) in FONT_FILE.chunks_exact(4).enumerate() {
            let reg = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            core::ptr::write_volatile((0x6800000 as *mut u32).add(i), reg);
        }
        let mut video_context = reboot_lib::VideoHardwareHandle::new();
        //setup 3d hardware
        VIDEO_HARDWARE.power_control.write(VideoPowerControl::all());
        VIDEO_HARDWARE.vram_control_bank_a.write(0x83); //map VRAM BANK A
        VIDEO_HARDWARE.vram_control_bank_e.write(0x83); //map VRAM BANK E
        VIDEO_HARDWARE.primary_display_control.write(
            PrimaryDisplayControl::BG_MODE_0
                | PrimaryDisplayControl::ENABLE_3D
                | PrimaryDisplayControl::ENABLE_BG_0,
        );
        VIDEO_HARDWARE.display_control_3d.write(1); //enables texture mapping
        video_context.next_frame(); //swap geometry buffers

        //init matricies
        video_context.init_matricies();
        VIDEO_HARDWARE
            .geometry_commands
            .select_matrix_stack(MatrixMode::POSITION);
        VIDEO_HARDWARE
            .geometry_commands
            .scale_matrix(0x1000, -0x1555, 0x1000);
        VIDEO_HARDWARE
            .geometry_commands
            .translate_matrix(-0x80 * 0x20, -0x58 * 0x20, 0);

        //more init
        VIDEO_HARDWARE
            .geometry_commands
            .pipeline_set_viewport
            .write(Viewport::WHOLE_SCREEN_DEFAULT);
        VIDEO_HARDWARE
            .geometry_commands
            .material_texture_attributes
            .write((7 << 20) | (2 << 26) | (1 << 29)); //bind font texture
        VIDEO_HARDWARE
            .geometry_commands
            .material_color_palette
            .write(0); //use color palette 0
        VIDEO_HARDWARE
            .geometry_commands
            .material_polygon_attributes
            .write(
                PolygonAttributes::RENDER_BACK_SURFACE
                    | PolygonAttributes::RENDER_FRONT_SURFACE
                    | PolygonAttributes::POLYGON_ALPHA_SOLID,
            );
        VIDEO_HARDWARE.clear_depth.write(0x7FFF); //max depth
                                                  //VIDEO_HARDWARE.clear_color.write(reboot_lib::Color::CONFIRM_GREEN); //greenish color

        reboot_lib::VideoTextPass::new(&mut video_context).text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str(TEST_STRING_UPPER);
            text_pass.next_line();
            text_pass.layout_str("Waiting on ARM7...");
        });
        video_context.next_frame();
        steal_main_mem();
        let nand_buffer = alloc::alloc::alloc(alloc::alloc::Layout::new::<[u32; 128]>());
        let nand_buffer = core::slice::from_raw_parts_mut(nand_buffer as *mut _, 1);
        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 0 {}
        reboot_lib::VideoTextPass::new(&mut video_context).text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str("ARM7 SUPER STOLEN!");
        });
        video_context.next_frame();

        read_encrypted_nand(nand_buffer, 0);

        reboot_lib::VideoTextPass::new(&mut video_context).text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str("COOL STUFF");
            text_pass.next_line();
            text_pass.next_line();

            let mbr = &*(nand_buffer as *mut [reboot_lib::StorageSector] as *const reboot_lib::StorageSector as *const () as *const mbr::MBR);//core::slice::from_raw_parts_mut(nand_buffer, 128);
            text_pass.layout_str(&alloc::format!("signature: {:x?}", &mbr.boot_signature));
            
            text_pass.next_line();

            for partition in &mbr.partitions {
                let lba = core::ptr::read_unaligned(core::ptr::addr_of!(partition.lba));
                let size = core::ptr::read_unaligned(core::ptr::addr_of!(partition.sector_count));
                text_pass.layout_str(&alloc::format!("partition: lba {:x?}, size {:x?}", lba, size));
                text_pass.next_line();
            }
           
        });
        video_context.next_frame();

        //core::ptr::write_volatile(0x5000000 as *mut u16, 0b0000111101010100);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0000111101010100);
        //core::ptr::write_volatile(0x5000000 as *mut u16, 0b1111100000000001);
    }
}
#[no_mangle]
pub unsafe extern "C" fn _start() {
    asm!(
        // Set up the stack pointer to 0x7C00
        "ldr sp, =0x37DF068",

        // Call the main function
        "bl {main}",

        // Halt the CPU after main returns (if it does)
        "2: b 2b", // Infinite loop

        main = sym main, // Link the `main` symbol
        options(noreturn) // No return possible from this function
    );
}
fn read_encrypted_nand(buffer: *mut [reboot_lib::StorageSector], start_sector: u32) {
    unsafe { 
        reboot_lib::arm9_set_buffer(buffer); 
        reboot_lib::arm9_read_nand_sector_encrypted(start_sector);
    }
}
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
