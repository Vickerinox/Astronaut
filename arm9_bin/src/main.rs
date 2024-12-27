#![no_main]
#![no_std]
#![feature(ptr_metadata)]

const FONT_FILE: &[u8] = include_bytes!("./font.bin");
const ARM7_BINARY: &[u8] = include_bytes!("./arm7.bin");
const BOOTSTRAP_BINARY: &[u8] = include_bytes!("./bootstrap.bin");

use core::arch::asm;

use reboot_lib::{
    Buttons, MatrixMode, PolygonAttributes, PrimaryDisplayControl, VideoPowerControl, Viewport, VIDEO_HARDWARE
};
extern crate alloc;

mod bootloader;
mod mbr;
mod nand;
pub mod new_takeover;
pub mod unlaunch;

pub unsafe fn nocash_write(str: &str) {
    const NOCASH_OUT_CHR: *mut u8 = 0x4fffa1c as *mut u8;
    for byte in str.as_bytes() {
        NOCASH_OUT_CHR.write_volatile(*byte);
    }
}

pub unsafe fn steal_main_mem() {
    reboot_lib::ALLOCATOR.init();
}
unsafe fn main() {
    unsafe {
        core::ptr::write_volatile(0x4000304 as *mut u32, 0b1000001110);

        //set background color to brat green.
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b1111100000111111);
        core::ptr::write_volatile(0x5000002 as *mut u16, 0xFFFF);
        core::ptr::write_volatile(0x5000004 as *mut u16, 0xFFFF);
        core::ptr::write_volatile(0x5000006 as *mut u16, 0xFFFF);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b1111100000111111);

        core::ptr::write_volatile(0x200_0000 as *mut u32, 0);

        reboot_lib::IPC_FIFO_HARDWARE.enable();
        reboot_lib::IPC_FIFO_HARDWARE.set_status(0);
        new_takeover::mysterious_takeover_function();

        core::ptr::write_volatile(0x04000240 as *mut u8, 0x80); //enable VRAM bank A

        //enable the 2D engine A, with no backgrounds on.
        core::ptr::write_volatile(0x4000000 as *mut u32, 0b0000000000000000100000011_0000_0000);
        core::ptr::write_volatile(0x4001000 as *mut u32, 0b0000000000000000100000000_0000_0000);

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
                | PrimaryDisplayControl::ENABLE_BG_0
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
        steal_main_mem();
        
        reboot_lib::VideoTextPass::new(&mut video_context).text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str("Waiting on ARM7...");
        });
        video_context.next_frame();
        

        let nand_buffer =
            core::slice::from_raw_parts_mut(0x2FFFE00 as *mut reboot_lib::StorageSector, 1);
        let sd_buffer =
            core::slice::from_raw_parts_mut(0x2FFFC00 as *mut reboot_lib::StorageSector, 1);
        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 0 {}
        let status = reboot_lib::IPC_FIFO_HARDWARE.recieve_raw_blocking();

        reboot_lib::VideoTextPass::new(&mut video_context).text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str("reading mbr...");
        });
        video_context.next_frame();

        read_sd_card(sd_buffer, 0x0);
        let sd_mbr = &*(sd_buffer as *mut [reboot_lib::StorageSector]
            as *const reboot_lib::StorageSector as *const ()
            as *const mbr::MBR);

        let sd_fs = if sd_mbr.has_valid_signature() {
            let sd_lba = core::ptr::read_unaligned(core::ptr::addr_of!(sd_mbr.partitions[0].lba));
            let sd_size =
                core::ptr::read_unaligned(core::ptr::addr_of!(sd_mbr.partitions[0].sector_count));
            nand::mount_sd_card_partition(sd_lba, sd_size, sd_buffer).ok()
        } else {
            None
        };


        read_encrypted_nand(nand_buffer, 0x0);

        let mbr = &*(nand_buffer as *mut [reboot_lib::StorageSector]
            as *const reboot_lib::StorageSector as *const ()
            as *const mbr::MBR);

        let nand_fs = if mbr.has_valid_signature() {
            let twl_lba = core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].lba));
            let twl_size =
                core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].sector_count));
                nand::mount_twl_main(twl_lba, twl_size, nand_buffer).ok()
        } else {
            None
        };

        let mut working_folder = if let Some(folder) = nand_fs.as_ref().or(sd_fs.as_ref()) {
            folder.root_dir()
        } else {
            panic!("No filesystem could be initialized, aborting...")
        };
        let mut showing = "Currently in: NAND";
        let mut old_controls;
        let mut new_controls = Buttons::empty();
        let mut index = 0usize;

        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0000111101010100);
        core::ptr::write_volatile(0x5000000 as *mut u16, 0);
        loop {
            old_controls = new_controls;
            new_controls = read_controller();

            let cont_controls = new_controls.intersection(!old_controls);
            let increment = if cont_controls.contains(Buttons::DIRECTION_DOWN) {
                1
            } else if cont_controls.contains(Buttons::DIRECTION_UP) {
                -1
            } else {
                0
            };
            if cont_controls.contains(Buttons::BUTTON_START) {
                if let Some(fs) = &sd_fs {
                    working_folder = fs.root_dir();
                    showing = "Currently in: SD CARD";
                }
            }
            if cont_controls.contains(Buttons::BUTTON_SELECT) {
                if let Some(fs) = &nand_fs {
                    working_folder = fs.root_dir();
                    showing = "Currently in: NAND";
                }
            }
            let select = cont_controls.contains(Buttons::BUTTON_A);
            let mut max = 0;
            let mut new_folder = None;

            reboot_lib::VideoTextPass::new(&mut video_context).text_pass(|text_pass| {
                text_pass.set_color(0x7FFF);
                text_pass.layout_str(&showing);
                text_pass.next_line();
                text_pass.next_line();
                for (num, item) in working_folder.iter().enumerate() {
                    text_pass.set_color(0x7FFF);
                    max = num;
                    match item {
                        Ok(item) => {
                            if num == index {
                                text_pass.layout_str(" > ");
                                if select {
                                    match alloc::str::from_utf8(item.short_file_name_as_bytes()) {
                                        Ok(a) => {
                                            new_folder = Some(alloc::string::String::from(a));
                                        }
                                        Err(_) => (),
                                    }
                                }
                            } else {
                                text_pass.layout_str("   ");
                            }
                            if item.is_dir() {
                                text_pass.set_color(0x7FF2);
                            } else if item.is_file() {
                                if is_bootable(item.short_file_name_as_bytes()) {
                                    text_pass.set_color(0x3FF4);
                                } else {
                                    text_pass.set_color(0x7FFF);
                                }
                            }
                            for byte in item.short_file_name_as_bytes() {
                                text_pass.layout_char(*byte);
                            }
                            text_pass.next_line();
                        }
                        Err(error) => {
                            text_pass.layout_str("ERROR");
                            text_pass.next_line();
                        }
                    }
                }
            });
            video_context.next_frame();
            index = index.saturating_add_signed(increment).clamp(0, max);
            if let Some(folder) = new_folder {
                let extension_point = folder.len() - 4;
                if folder.is_char_boundary(extension_point) {
                    if is_bootable(folder.as_bytes()) {
                        match working_folder.open_file(&folder) {
                            Ok(file) => match bootloader::boot_app(file) {
                                Ok(()) => unreachable!(),
                                Err(_) => (),
                            },
                            Err(_) => (),
                        }
                    }
                }

                match working_folder.open_dir(&folder) {
                    Ok(ok) => {
                        working_folder = ok;
                    }
                    Err(_) => (),
                }
            }
        }
    }
}

pub fn is_bootable(str: &[u8]) -> bool {
    let len = str.len() - 4;
    &str[len..] == b".APP" || &str[len..] == b".NDS" || &str[len..] == b".DSI"
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
fn read_sd_card(buffer: *mut [reboot_lib::StorageSector], start_sector: u32) {
    unsafe {
        reboot_lib::arm9_set_buffer(buffer);
        reboot_lib::arm9_read_sd_sector(start_sector);
    }
}
fn read_controller() -> Buttons {
    unsafe {
        reboot_lib::arm9_send_controller_read();
        let bits = reboot_lib::IPC_FIFO_HARDWARE.recieve_raw_blocking();
        Buttons::from_bits_retain(bits as u16)
    }
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b0000000000111111);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0000000000111111);

        let mut video_context = reboot_lib::VideoHardwareHandle::new();

        video_context.next_frame();
        reboot_lib::VideoTextPass::new(&mut video_context).text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str("Panic occured:");
            text_pass.next_line();
            text_pass.next_line();
            text_pass.layout_str(&alloc::format!("message: {}",  info.message()));
            text_pass.next_line();
            text_pass.next_line();
            if let Some(loc) = info.location() {
                text_pass.layout_str(&alloc::format!("location: {}",  loc));
            }
        });
        video_context.next_frame();
    }
    loop {}
}
