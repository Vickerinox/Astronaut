#![no_main]
#![no_std]
#![feature(ptr_metadata)]
#![feature(str_from_utf16_endian)]

const FONT_FILE: &[u8] = include_bytes!("./font.bin");
const ARM7_BINARY: &[u8] = include_bytes!("./arm7.bin");
const BOOTSTRAP_BINARY: &[u8] = include_bytes!("./bootstrap.bin");

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::{self, Vec},
};
use core::{alloc::Layout, arch::asm, ptr::addr_of};
use gui::VideoTextPass;
use micro_imgui::{
    Backend, Color, Sizing, Vec2, widgets::{button::Button, label::Label}
};
use reboot_lib::{
    VideoHardware, VideoHardwareHandle, arm9_send_arm7_jump, fatfs::{TimeProvider, Dir, FileSystem, LossyOemCpConverter}, flush_mmc, sound::SOUND_HARDWARE
};
use reboot_lib::{
    fatfs::{Read, Seek, SeekFrom},
    music_modules::mods::{MODAsyncLoader, MODHeader},
};
use reboot_lib::{
    spi::firmware::{FirmwareHeader, UserData},
    Buttons, MatrixMode, PolygonAttributes, PrimaryDisplayControl, StorageSector,
    VideoPowerControl, Viewport, IPC_FIFO_HARDWARE, VIDEO_HARDWARE,
};

use crate::{
    gui::{DSMicroGuiBackend, Input},
    nand::BasicSDMMCCursor,
};
use common::bootstrap::HeaderNDS;

extern crate alloc;

mod bootloader;
mod gui;
mod mbr;
mod nand;
pub mod new_takeover;

pub unsafe fn nocash_write(str: &str) {
    const NOCASH_OUT_CHR: *mut u8 = 0x4fffa1c as *mut u8;
    for byte in str.as_bytes() {
        NOCASH_OUT_CHR.write_volatile(*byte);
    }
}

/// A interrupt handler appropriate for the ds, courtesy of libnds
unsafe fn interrupt_handler() {
    // what you are about to see is probably the most unoxidized code i've ever written -vikrinox
    core::arch::asm!(
        // According to libnds, r0-r3, as well as r12 and lr are saved by the BIOS handler. (2025-12-04: This is true)
        "mov r12, {i_base}",
        "ldr r1, [r12, {i_e}]",
        "ldr r2, [r12, {i_f}]",
        "ands r1, r1, r2", //the interrupt bits to be serviced! (i.e IE & IF)
        "moveq pc, lr", // EARLY RETURN: no interrupts to service

        // Get the bit index for the "highest priority" IRQ
        "clz r0, r1",
        "rsb r0, r0, #31",  //find the higest non-zero bit by counting zeros
        "mov r1, #1",
        "mov r1, r1, lsl r0", //create a "bitmask" of the IRQ

        // Clear the interrupt on the hardware side
        "str r1, [r12, {i_f}]",

        // Clear the interrupt on the bios side
        "ldr r2, ={bios_f}",
        "ldr r3, [r2]",
        "orr r3, r3, r1",
        "str r3, [r2]",

        // load irq table and jump to funciton pointer
        "ldr r3, ={irq_table}",
        "add r3, r0, lsl #2",
        "ldr r3, [r3]",
        "cmp r3, #0",
        "beq 2f", //EARLY RETURN: no interrupt handler installed
            //set IME = 0
            "ldr r1, [r12, {ime}]",
            "str r12, [r12, {ime}]", //HACK: IME only cares about bit 0, so this sets IME = 0

            //get into system mode
            "mrs r0, spsr",
            "push {{r0,r1,r12,lr}}", // {spsr, ime, i_base, irq_lr}
            "mrs r0, cpsr",
            "bic r1, r0, {user_clear}",
            "orr r1, r1, {user_set}",
            "msr cpsr, r1",

            //run the interrupt handler
            "push {{r0, lr}}", // NOTE: we push LR *again* since system mode has it's own lr.
            "blx r3",         //execute interrupt handler (the moment we've been waiting for!!!)
            "pop {{r0, lr}}",

            //Hop out of system mode
            "msr cpsr, r0",
            "pop {{r0,r1,r12,lr}}", // {spsr, ime, i_base, irq_lr}
            "msr spsr, r0",

            //Restore IME
            "str r1, [r12, {ime}]",
        //return
        "2:",

        i_base = const 0x0400_0000, //register base
        i_e = const 0x210,  //interrupt enable register
        i_f = const 0x214,  //interrupt request register
        bios_f = const 0x2fe3ff8,   //interrupt request regiser (BIOS)
        irq_table = sym INTERRUPT_TABLE,
        ime = const 0x208,  //master interrupt enable
        user_clear = const 0x80 | 0x40 | 0x1F, //disable IRQ/FIQ masking, clear mode bits
        user_set = const 0x1F,  //Set mode to "System"
    );
}

static mut INTERRUPT_TABLE: [*mut fn(); 32] = [core::ptr::null_mut(); 32];
static mut FRAME_COUNTER: usize = 0;
pub unsafe fn steal_main_mem() {
    reboot_lib::ALLOCATOR.init();
}
static mut PLAYING_MOD: *mut MODHeader = core::ptr::null_mut();
fn play_mod() {}
fn vblank_interrupt() {
    unsafe { FRAME_COUNTER += 1 };
}

pub struct StaticReader(&'static [u8]);
#[derive(Debug)]
pub enum StaticReaderError {
    OutOfData,
    UnexpectedEof,
    WriteZero,
}
impl reboot_lib::fatfs::IoBase for StaticReader {
    type Error = StaticReaderError;
}
impl reboot_lib::fatfs::IoError for StaticReaderError {
    fn is_interrupted(&self) -> bool {
        false
    }

    fn new_unexpected_eof_error() -> Self {
        Self::UnexpectedEof
    }

    fn new_write_zero_error() -> Self {
        Self::WriteZero
    }
}
impl reboot_lib::fatfs::Read for StaticReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.len() > self.0.len() {
            return Err(StaticReaderError::OutOfData);
        }
        let (chunk, new_self) = self.0.split_at(buf.len());
        buf.copy_from_slice(chunk);
        self.0 = new_self;
        Ok(buf.len())
    }
}
unsafe fn init_font() {
    for (i, w) in FONT_FILE.chunks_exact(4).enumerate() {
        let reg = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
        core::ptr::write_volatile((0x6800000 as *mut u32).add(i), reg);
    }
}
unsafe fn init_3d_hardware(video_context: &mut VideoHardwareHandle) {
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
        .scale_matrix(0x1000, -0x1555, -0x1000);
    VIDEO_HARDWARE
        .geometry_commands
        .scale_matrix(0x2000, 0x2000, 0x2000);

    VIDEO_HARDWARE
        .geometry_commands
        .translate_matrix(-0x80 * 0x10, -0x60 * 0x10, 100);

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
}

unsafe fn try_mount_sd() -> Option<FileSystem<BasicSDMMCCursor<'static>>> {
    let sd_buffer = core::slice::from_raw_parts_mut(0x2FE8000 as *mut StorageSector, 1);
    read_sd_card(sd_buffer, 0).ok()?;
    let mbr: &mbr::MBR = &*(transmute_slice(sd_buffer));
    if !mbr.has_valid_signature() {
        return None;
    }
    let twl_lba = core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].lba));
    let sd_buffer =
        core::slice::from_raw_parts_mut(0x2FE8000 as *mut reboot_lib::StorageSector, 16);
    nand::mount_sd(twl_lba, 0, sd_buffer).ok()
}
unsafe fn try_mount_nand() -> Option<FileSystem<BasicSDMMCCursor<'static>>> {
    let nand_buffer = core::slice::from_raw_parts_mut(0x2FEC000 as *mut StorageSector, 1);
    read_encrypted_nand(nand_buffer, 0).ok()?;
    let mbr: &mbr::MBR = &*(transmute_slice(nand_buffer));
    if !mbr.has_valid_signature() {
        return None;
    }
    let twl_lba = core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].lba));
    read_encrypted_nand(nand_buffer, twl_lba).ok()?;
    let twl_size = core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].sector_count));
    let nand_buffer =
        core::slice::from_raw_parts_mut(0x2FEC000 as *mut reboot_lib::StorageSector, 16);
    nand::mount_twl_main(twl_lba, twl_size, nand_buffer).ok()
}

pub struct RebootState {
    current_path: String,
    
}
fn populate_fs_vec<T: TimeProvider>(folder: &Dir<'_, BasicSDMMCCursor<'static>, T, LossyOemCpConverter>) -> Vec<(String, bool, Color)> {
let mut vec: Vec<_> = folder.iter()
                .filter_map(|folder| {
                    let item = folder.ok()?;
                    let name = alloc::string::String::from(alloc::str::from_utf8(item.short_file_name_as_bytes()).ok()?);
                    if name == "." {
                        return None;
                    }
                    let color = if item.is_dir() {
                        Color::new(200, 100, 100)
                    } else {
                        if is_bootable(item.short_file_name_as_bytes()) {
                            Color::new(100, 200, 100)
                        } else if is_music_module(item.short_file_name_as_bytes()){
                            Color::new(100, 100, 200)
                        } else {
                            Color::new(180, 180, 180)
                        }
                    };
                    Some((name, item.is_dir(), color))
                })
                .collect();
vec.sort_by(|(_, b,c), (_,d,e)| core::cmp::Ord::cmp(&c.0, &e.0));
    vec
}
unsafe fn main() {
    unsafe {
        core::ptr::write_volatile(0x4000304 as *mut u32, 0b1000001110);

        (0x4000204 as *mut u16).write_volatile((1 << 15) | (1 << 13));

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
        core::ptr::write_volatile(
            0x4000000 as *mut u32,
            0b00000000000000001_0000_0001_0000_0_000,
        );
        core::ptr::write_volatile(
            0x4001000 as *mut u32,
            0b00000000000000001_0000_0000_0000_0_000,
        );

        core::ptr::write_volatile(0x04000244 as *mut u8, 0x80); //enable VRAM bank E

        //write to "color palette 0"
        core::ptr::write_volatile(0x06880000 as *mut u16, 0b0_00000_00000_00000);
        core::ptr::write_volatile(0x06880004 as *mut u16, 0b0_00000_00000_00000);
        core::ptr::write_volatile(0x06880002 as *mut u16, 0b0_11111_11111_11111);
        core::ptr::write_volatile(0x06880006 as *mut u16, 0b0_00000_00000_11111);

        //copy font to vram
        init_font();
        let mut video_context = reboot_lib::VideoHardwareHandle::new();
        init_3d_hardware(&mut video_context);
        steal_main_mem();
        const SCREEN_RECT: micro_imgui::Rect = micro_imgui::Rect {
            min: Vec2::ZERO,
            max: Vec2::new(255, 191),
        };
        

        gui::VideoTextPass::new(&mut video_context, SCREEN_RECT).text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str("GURU MEDITAION ERROR", 8);
            text_pass.next_line();
            text_pass.layout_str(
                "startup is stuck at waiting for the co-CPU, meaning the exploit has failed.",
                8,
            );
            text_pass.next_line();
            text_pass.layout_str(
                "try restarting the console, or reach out to the dsi hacking server",
                8,
            );
        });
        
        video_context.next_frame();
        

    

        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 1 {}
        reboot_lib::IPC_FIFO_HARDWARE.set_status(1);
        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 0 {}
        reboot_lib::IPC_FIFO_HARDWARE.set_status(0);
        
        let mut dtcm: u32 = 0x2FE_000A;

        core::arch::asm!(
            "mcr p15, 0, {0}, c9, c1, 0",
            "mrc p15, 0, {0}, c9, c1, 0",
            inout(reg) dtcm,
        );
        
        core::ptr::write_volatile(0x4000304 as *mut u32, 0b1000001111);
        irq_init();
        
        IPC_FIFO_HARDWARE.enable_recv_irq();
        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::IPCNonEmpty);
        //reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::VBlank);
        core::ptr::write_volatile(0x04000004 as *mut u16, 0xFFFF);
        //loop {}


        let check_sd = IPC_FIFO_HARDWARE.recieve_raw_blocking() == 1;
        let check_nand = IPC_FIFO_HARDWARE.recieve_raw_blocking() == 1;
        let sd_fs = if check_sd { try_mount_sd() } else { None };
        let nand_fs = if check_nand { try_mount_nand() } else { None };

        video_context.next_frame();

        ready_arm7();
        
        let mut loading_mod_file = None;
        
        let (mut working_folder, mut current_path) = if let Some(folder) = sd_fs.as_ref() {
            let root = folder.root_dir();

            match root.open_file("/_nds/vlaunch/default.mod") {
                Ok(file) => {
                    stop_mod_file();
                    loading_mod_file = Some(MODAsyncLoader::new(file));
                }
                Err(_abort) => {
                    start_procedural_music();
                }
            }

            (Some(root), String::from("sd:/"))
        } else {
            start_procedural_music();
            (None, String::new())
        };
        let mut booting_app: Option<(
            reboot_lib::fatfs::File<
                '_,
                nand::BasicSDMMCCursor<'_>,
                _,
                reboot_lib::fatfs::LossyOemCpConverter,
            >,
            Vec<u8>,
        )> = None;
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0000111101010100);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0001000010000100);
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b0001000010000100);

        video_context.next_frame();

        let mut in_sd = true;
        let backend = gui::DSMicroGuiBackend::new(video_context);
        let mut folders: Vec<_> = if let Some(fs) = &mut working_folder {
            populate_fs_vec(&fs)
        } else {
            alloc::vec![]
        };
        micro_imgui::run(backend, (), |f, _| {
            f.central_panel(|ui| {
                
                if in_sd {
                    ui.header("SD Card view:");
                } else {
                    ui.header("NAND view:");
                }
                let mut new_folder = None;
                if ui.input_pressed(gui::Input(Buttons::BUTTON_L)) || ui.input_pressed(gui::Input(Buttons::BUTTON_R)) {
                    
                    if in_sd && nand_fs.is_some() {
                        if let Some(root) = nand_fs.as_ref() {
                            new_folder = Some(root.root_dir());
                            current_path = String::from("nand:/");
                            in_sd = false;
                        }       
                    } else if !in_sd && sd_fs.is_some() {
                        if let Some(root) = sd_fs.as_ref() {
                            new_folder = Some(root.root_dir());
                            current_path = String::from("sd:/");
                            in_sd = true;
                        } 
                    }
                }
                if let Some(working_folder) = &mut working_folder {
                    
                    if let Some(loading_mod) = loading_mod_file.take() {
                        let (progress, max) = loading_mod.progress();
                        let progress_bar = progress * 27 / max;
                        let bar = (0..27)
                            .map(|i| if i < progress_bar { '=' } else { '.' })
                            .collect::<alloc::string::String>();
                        ui.label(format!("Loading song [{bar}]"));
                        ui.request_repaint();
                        match loading_mod.process() {
                            Ok(Some(ret)) => {
                                send_mod_file(ret);
                            }
                            Ok(None) => (),
                            Err(cont) => loading_mod_file = Some(cont),
                        }
                    } else {
                        ui.label(&current_path);
                    }

                    if let Some((mut file, mut header)) = booting_app.take() {
                        let head = &mut *(&mut header[..] as *mut [u8] as *mut u8
                            as *mut HeaderNDS);
                        ui.label("                 Title info:");
                        ui.label(alloc::format!(
                            "      Name: {} TID: {:08X}",
                            core::str::from_utf8(&head.title).unwrap_or("UNKNOWN"),
                            head.tid
                        ));
                        ui.label(" ");
                        ui.label("ARM9 offsets:");
                        ui.label(alloc::format!("entry: {:08X}", head.arm9_entry));
                        ui.label(alloc::format!(
                            "load: {:08X}, size: {:08X}",
                            head.arm9_load,
                            head.arm9_size
                        ));
                        ui.label(" ");
                        ui.label("ARM7 offsets:");
                        ui.label(alloc::format!("entry: {:08X}", head.arm7_entry));
                        ui.label(alloc::format!(
                            "load: {:08X}, size: {:08X}",
                            head.arm7_load,
                            head.arm7_size
                        ));
                        ui.label("ARMi offsets:");
                        ui.label(alloc::format!(
                            "7i: {:08X} {:08X}",
                            head.arm7i_load,
                            head.arm7i_size
                        ));
                        ui.label(alloc::format!(
                            "9i: {:08X} {:08X}",
                            head.arm9i_load,
                            head.arm9i_size
                        ));
                        ui.label(" ");
                        
                        ui.label(alloc::format!(
                            "mbk: {:08X} {:08X} {:08X} {:08X} {:08X} | {:08X} {:08X} {:08X} | {:08X} {:08X} {:08X} ",
                            head.global_mbks[0], head.global_mbks[1], head.global_mbks[2],
                            head.global_mbks[3], head.global_mbks[4],
                            head.arm9_mbks[0], head.arm9_mbks[1], head.arm9_mbks[2],
                            head.arm7_mbks[0], head.arm7_mbks[1], head.arm7_mbks[2],
                        ));
                        ui.label(" ");
                        ui.horizontal(|ui| {
                            if ui.button("Launch!!").clicked() {
                                
                                match file.seek(SeekFrom::Start(0)) {
                                    Ok(0) => {
                                        bootloader::boot_app(file, &current_path);
                                    }
                                    Ok(_what) => (),
                                    Err(_error) => (),
                                }
                            } else {
                                if ui.button("Go back").clicked() {
                                    while current_path.pop() != Some('/') {}
                                    current_path.push('/');
                                    booting_app = None;
                                } else {
                                    booting_app = Some((file, header));
                                }
                            }
                        });
                    } else {
                        
                        for item in folders.iter() {
                            if ui
                                .add(Button::new(&item.0, Sizing::Padded(Vec2::new(248, 8)), item.2))
                                .clicked()
                            {
                                if item.1 {
                                    match working_folder.open_dir(&item.0) {
                                        Ok(folder) => {
                                            if &item.0 == "." {}
                                            else if &item.0 == ".." {
                                                current_path.pop();
                                                while current_path.pop() != Some('/') {}
                                                current_path.push('/');
                                            }
                                            else {
                                                current_path.push_str(&item.0);
                                                current_path.push('/');
                                            }
                                            new_folder = Some(folder)
                                        },
                                        Err(_) => (),
                                    }
                                } else {
                                    let extension_point = item.0.len() - 4;
                                    if item.0.is_char_boundary(extension_point) {
                                        if is_bootable(item.0.as_bytes()) {
                                            match working_folder.open_file(&item.0) {
                                                Ok(mut file) => {
                                                    current_path.push_str(&item.0);
                                                    let mut header_buffer = alloc::vec![0u8; 4096];
                                                    file.read(&mut header_buffer);
                                                    booting_app = Some((file, header_buffer));
                                                }
                                                Err(_) => (),
                                            }
                                        } else if is_music_module(item.0.as_bytes()) {
                                            match working_folder.open_file(&item.0) {
                                                Ok(module) => {
                                                    loading_mod_file =
                                                        Some(MODAsyncLoader::new(module));
                                                    drop(stop_mod_file());
                                                }
                                                Err(_abort) => (),
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(new_folder) = new_folder {
                            folders = populate_fs_vec(&new_folder);
                            *working_folder = new_folder;
                        }
                    }
                }
            });
        });
    }
}

pub fn is_bootable(str: &[u8]) -> bool {
    let len = str.len() - 4;
    &str[len..] == b".APP" || &str[len..] == b".NDS" || &str[len..] == b".DSI"
}
pub fn is_music_module(str: &[u8]) -> bool {
    let len = str.len() - 4;
    &str[len..] == b".MOD"
}

const DSI_WRAM_START: usize = 0x037C0000;
#[no_mangle]
pub unsafe extern "C" fn _start() {
    asm!(
        //turn of interrupts via the IME register
        "mov r0, #0x04000000",
        "str r0, [r0, #0x208]",

        //load start of stack(s)
        "mov r0, #0x12",
        "msr cpsr, r0",
        "ldr sp, ={stack_irq}",

        "mov r0, #0x13",
        "msr cpsr, r0",
        "ldr sp, ={stack_svc}",

        "mov r0, #0x1F",
        "msr cpsr, r0",
        "ldr sp, ={stack_sys}",


        // Call the main function
        "bl {main}",

        // Halt the CPU after main returns (if it does)
        "2: b 2b", // Infinite loop
        stack_irq = const DSI_WRAM_START + 0x1000,
        stack_svc = const DSI_WRAM_START + 0x2000,
        stack_sys = const DSI_WRAM_START + 0x3000,
        main = sym main, // Link the `main` symbol
        options(noreturn) // No return possible from this function
    );
}
unsafe fn ready_arm7() {
    reboot_lib::arm9_ready_arm7();
}
fn send_mod_file(module: Box<MODHeader>) -> Option<Box<MODHeader>> {
    unsafe {
        match reboot_lib::arm9_send_arm7(0, Box::into_raw(module) as *mut ()) {
            Ok(_) => None,
            Err(old_mod) => Some(Box::from_raw(u32::from(old_mod) as *mut MODHeader)),
        }
    }
}
fn stop_mod_file() -> Option<Box<MODHeader>> {
    unsafe {
        match reboot_lib::arm9_send_arm7(0, core::ptr::null_mut()) {
            Ok(_) => None,
            Err(old_mod) => Some(Box::from_raw(u32::from(old_mod) as *mut MODHeader)),
        }
    }
}
fn start_procedural_music() {
    unsafe {
        reboot_lib::arm9_send_arm7(1, core::ptr::null_mut());
    }
}
fn read_encrypted_nand(
    buffer: *mut [reboot_lib::StorageSector],
    start_sector: u32,
) -> Result<(), u32> {
    unsafe {
        flush_mmc();
        reboot_lib::arm9_set_buffer(buffer)?;
        reboot_lib::arm9_read_nand_sector_encrypted(start_sector)?;
        flush_mmc();
        flush_mmc();
    }
    Ok(())
}
fn read_sd_card(buffer: *mut [reboot_lib::StorageSector], start_sector: u32) -> Result<(), u32> {
    unsafe {
        flush_mmc();
        reboot_lib::arm9_set_buffer(buffer)?;
        reboot_lib::arm9_read_sd_sector(start_sector)?;
        flush_mmc();
        flush_mmc();
    }
    Ok(())
}
pub fn read_controller() -> Buttons {
    unsafe { reboot_lib::arm9_send_controller_read() }
    
}
fn read_firmware(buffer: *mut [reboot_lib::StorageSector], start_offset: u32) {
    unsafe {
        reboot_lib::arm9_set_buffer(buffer);
        reboot_lib::arm9_read_firmware(start_offset);
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
        gui::VideoTextPass::new(
            &mut video_context,
            micro_imgui::Rect::from_min_size(Vec2::ZERO, Vec2::new(255, 191)),
        )
        .text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.layout_str(
                "Panic occured, Under normal circumstances this should not happen.",
                8,
            );
            /* 
            text_pass.next_line();
            text_pass.next_line();
            text_pass.layout_str(&alloc::format!("message: {}", info.message()), 8);
            text_pass.next_line();
            text_pass.next_line();
            if let Some(loc) = info.location() {
                text_pass.layout_str(&alloc::format!("location: {}", loc), 8);
            }
            */
        });
        video_context.next_frame();
    }
    loop {}
}

unsafe fn transmute_slice<T, U>(slice: *mut [T]) -> *mut U {
    slice as *mut T as *mut () as *mut U
}
unsafe fn irq_init() {
    use reboot_lib::INTERUPT_HARDWARE;
    INTERUPT_HARDWARE.master.write(0);
    INTERUPT_HARDWARE.enable.write(0);
    INTERUPT_HARDWARE.request.write(!0);
    (0x02FE_3FFC as *mut unsafe fn()).write(interrupt_handler);
    INTERUPT_HARDWARE.master.write(1);
}
