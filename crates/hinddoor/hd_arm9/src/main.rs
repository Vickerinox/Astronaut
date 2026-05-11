#![no_main]
#![no_std]
#![feature(ptr_metadata)]
#![feature(str_from_utf16_endian)]
#![feature(str_from_raw_parts)]

const ARM7_BINARY: &[u8] = include_bytes!("./arm7.bin");
const BOOTSTRAP_BINARY: &[u8] = include_bytes!("./bootstrap.bin");

use alloc::{boxed::Box, string::String, vec::Vec};
use core::arch::asm;
use core::ops::Div;
use core::str;

use micro_imgui::{Color, Vec2};
use reboot_lib::music_modules::mods::MODHeader;
use reboot_lib::{arm9_check_sdmmc, arm9_init_sdmmc, flush_mmc, VideoHardwareHandle};
use reboot_lib::{
    Buttons, MatrixMode, PolygonAttributes, PrimaryDisplayControl, StorageSector,
    VideoPowerControl, Viewport, IPC_FIFO_HARDWARE, VIDEO_HARDWARE,
};

use fatfs_embedded::fatfs::diskio::{DiskResult, FatFsDriver, IoctlCommand};
static mut SD_ERROR: u32 = 0;
static mut EMMC_ERROR: u32 = 0;
pub struct SDMMCDriver {
    nand_controller: Option<BasicSDMMCCursor<'static>>,
    sd_controller: Option<BasicSDMMCCursor<'static>>,
}
impl FatFsDriver for SDMMCDriver {
    fn disk_status(&mut self, drive: u8) -> u8 {
        match unsafe { arm9_check_sdmmc(drive) } {
            Ok(()) => 0,
            Err(_any) => 1,
        }
    }
    fn disk_initialize(&mut self, drive: u8) -> u8 {
        match unsafe { arm9_init_sdmmc(drive) } {
            Ok(()) => match drive {
                1 => {
                    self.sd_controller = unsafe { try_mount_sd() };
                    self.sd_controller.is_none() as u8
                }
                2 => {
                    self.nand_controller = unsafe { try_mount_nand() };
                    self.nand_controller.is_none() as u8
                }
                _ => 1,
            },
            Err(bits) => {
                match drive {
                    1 => unsafe {
                        SD_ERROR = bits.get();
                    },
                    2 => unsafe {
                        EMMC_ERROR = bits.get();
                    },
                    _ => (),
                }
                1
            },
        }
    }
    fn disk_ioctl(&mut self, data: &mut IoctlCommand) -> DiskResult {
        match data {
            IoctlCommand::CtrlSync(_) => {
                if let Some(flusha) = &mut self.sd_controller {
                    flusha.flush();
                }
                DiskResult::Ok
            }
            IoctlCommand::GetSectorCount(_) => DiskResult::ParameterError,
            IoctlCommand::GetSectorSize(_) => DiskResult::ParameterError,
            IoctlCommand::GetBlockSize(_) => DiskResult::ParameterError,
        }
    }
    fn disk_read(&mut self, drive: u8, mut buffer: &mut [u8], sector: u32) -> DiskResult {
        let Some(controller) = (match drive {
            1 => &mut self.sd_controller,
            2 => &mut self.nand_controller,
            _ => return DiskResult::ParameterError,
        }) else {
            return DiskResult::NotReady;
        };
        let new_pos = sector;
        if controller.seek(new_pos) != Ok(new_pos) {
            return DiskResult::NotReady;
        }
        while !buffer.is_empty() {
            let Ok(progress) = controller.read(buffer) else {
                return DiskResult::NotReady;
            };
            let Some(remaining_buffer) = buffer.get_mut(progress..) else {
                return DiskResult::Error;
            };
            buffer = remaining_buffer;
        }
        DiskResult::Ok
    }
    fn disk_write(&mut self, drive: u8, mut buffer: &[u8], sector: u32) -> DiskResult {
        let Some(controller) = (match drive {
            1 => &mut self.sd_controller,
            2 => return DiskResult::WriteProtected, //&mut self.nand_controller,
            _ => return DiskResult::ParameterError,
        }) else {
            return DiskResult::NotReady;
        };
        let new_pos = sector;
        if controller.seek(new_pos) != Ok(new_pos) {
            return DiskResult::NotReady;
        }
        while !buffer.is_empty() {
            let Ok(progress) = controller.write(buffer) else {
                return DiskResult::NotReady;
            };
            let Some(remaining_buffer) = buffer.get(progress..) else {
                return DiskResult::Error;
            };
            buffer = remaining_buffer;
        }
        DiskResult::Ok
    }
}

use crate::gui::{TextLayoutHandle, VideoTextPass};
use crate::nand::BasicSDMMCCursor;

extern crate alloc;

mod autoboot;
mod boot;
pub mod fat;
mod gui;
mod mbr;
mod nand;
pub mod new_takeover;
#[repr(C)]
pub struct NandAutobootEntry {
    category: u32,
    title_id: u32,
    version: u32,
    buttons: reboot_lib::Buttons,
    _reserved: u16,
}
impl NandAutobootEntry {
    pub const EMPTY: NandAutobootEntry = NandAutobootEntry {
        category: 0,
        title_id: 0,
        version: 0,
        buttons: Buttons::empty(),
        _reserved: 0,
    };
}
static mut NAND_AUTOBOOTS: [NandAutobootEntry; 40] = [NandAutobootEntry::EMPTY; 40];

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

pub unsafe fn steal_main_mem() {
    reboot_lib::ALLOCATOR.init();
}

unsafe fn init_font() {
    /*
    const FONT_FILE: &[u8] = include_bytes!("./font.bin");
    for (i, w) in FONT_FILE.chunks_exact(4).enumerate() {
        let reg = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
        core::ptr::write_volatile((0x6800000 as *mut u32).add(i), reg);
    }
    */

    #[cfg(target_arch = "arm")]
    {
        const FONT_FILE: &[u8] = include_bytes!("./font_compressed.bin");
        for (i, w) in FONT_FILE.iter().enumerate() {
            core::ptr::write_volatile((0x2000800 as *mut u8).add(i), *w);
        }
        core::arch::asm!(
            "SWI 0x110000",
            in("r0") 0x2000800,
            in("r1") 0x2000000,
            lateout("r0") _,
            lateout("r1") _,
            out("r2") _,
            out("r3") _,
        );
        for i in 0..0x200 {
            let reg = core::ptr::read_volatile((0x200_0000 as *const u32).add(i));
            core::ptr::write_volatile((0x6800000 as *mut u32).add(i), reg);
        }
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

unsafe fn try_mount_sd() -> Option<BasicSDMMCCursor<'static>> {
    let sd_buffer = core::slice::from_raw_parts_mut(0x2FE8000 as *mut StorageSector, 1);
    read_sd_card(sd_buffer, 0).ok()?;
    let mbr: &mbr::MBR = &*(transmute_slice(sd_buffer));
    if !mbr.has_valid_signature() {
        return None;
    }
    let twl_lba = core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].lba));
    let sd_buffer =
        core::slice::from_raw_parts_mut(0x2FE8000 as *mut reboot_lib::StorageSector, 32);
    match BasicSDMMCCursor::new(sd_buffer, twl_lba, false) {
        Ok(succ) => Some(succ),
        Err(code) => { SD_ERROR = code; None},
    }
}

unsafe fn try_mount_nand() -> Option<BasicSDMMCCursor<'static>> {
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
        core::slice::from_raw_parts_mut(0x2FEC000 as *mut reboot_lib::StorageSector, 32);
    match BasicSDMMCCursor::new(nand_buffer, twl_lba, true) {
         Ok(succ) => Some(succ),
        Err(code) => { EMMC_ERROR = code; None},
    }
}

static mut CHECK_NAND: bool = false;
static mut CHECK_SD: bool = false;
pub struct RebootState {
    current_path: String,
}
use fatfs_embedded::fatfs::{FS_NAND, FS_SD};
const COLOR_BOOTABLE: Color = Color::new(100, 200, 100);
const COLOR_MUSIC: Color = Color::new(100, 100, 200);
fn populate_fs_vec(folder: &mut fatfs_embedded::fatfs::Directory) -> Vec<(String, String, bool, Color)> {
    let mut vec: Vec<_> = alloc::vec::Vec::new();
    unsafe {
        loop {
            if let Ok(file) = fatfs_embedded::readdir(folder) {
                let Ok(name) = core::ffi::CStr::from_ptr(file.fname.as_ptr()).to_str() else {
                    continue;
                };
                let mut name = alloc::string::String::from(name);
                if name.is_empty() {
                    break;
                }
                let is_dir =
                    file.fattrib & fatfs_embedded::fatfs::FileAttributes::Directory.bits() > 0;
                let color = if is_dir {
                    Color::new(200, 100, 100)
                } else {
                    let s_name = core::ffi::CStr::from_ptr(file.altname.as_ptr()).to_bytes();
                    let s_name = if s_name.is_empty() {
                        name.as_bytes()
                    } else {
                        s_name
                    };
                    if is_bootable(&s_name) {
                        COLOR_BOOTABLE
                    } else if is_music_module(&s_name) {
                        COLOR_MUSIC
                    } else {
                        Color::new(180, 180, 180)
                    }
                };
                let fname = name.clone();
                if name.len() > 35 {
                    let mut boundary = 32;
                    while !name.is_char_boundary(boundary) {
                        boundary += 1;
                    }
                    name.split_off(boundary);
                    name.push_str("...");
                }
                vec.push((name, fname, is_dir, color))
            } else {
                panic!("SD WAS EJECTED!");
            }
        }
    }
    for i in 1..vec.len() {
        let Some(temp) = vec.get(i) else { break };
        let temp = temp.clone();
        let mut j = i;
        loop {
            let Some(under) = vec.get(j - 1) else { break };
            if under.3 .0 > temp.3 .0 {
                let under = under.clone();
                let Some(over) = vec.get_mut(j) else { break };
                *over = under;
                j -= 1;
            } else {
                break;
            }
        }
        vec[j] = temp;
    }

    vec
}
unsafe fn base_init_graphics() {

}
unsafe fn main() {
    unsafe {
        core::ptr::write_volatile(0x4000304 as *mut u32, 0b1000001110);

        (0x4000204 as *mut u16).write_volatile((1 << 15) | (1 << 13));

        //set background color to brat green.
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b1111100000111111);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b1111100000111111);

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
            text_pass.layout_str("If you can see this screen, the Co-CPU (ARM7) has yet to respond. And most likely the console has crashed.", 8);
        });

        video_context.next_frame();

        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 1 {}
        reboot_lib::IPC_FIFO_HARDWARE.set_status(1);
        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 0 {}
        reboot_lib::IPC_FIFO_HARDWARE.set_status(0);

        #[cfg(target_arch = "arm")]
        {
            let mut dtcm: u32 = 0x2FE_000A;

            core::arch::asm!(
                "mcr p15, 0, {0}, c9, c1, 0",
                "mrc p15, 0, {0}, c9, c1, 0",
                inout(reg) dtcm,
            );
        }

        core::ptr::write_volatile(0x4000304 as *mut u32, 0b1000001111);
        irq_init();

        IPC_FIFO_HARDWARE.enable_recv_irq();
        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::IPCNonEmpty);
        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::VBlank);
        core::ptr::write_volatile(0x04000004 as *mut u16, 0xFFFF);

        video_context.next_frame();

        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0000111101010100);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0001000010000100);
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b0001000010000100);

        video_context.next_frame();

        fatfs_embedded::fatfs::diskio::install(&mut SDMMC_DRIVER);

        let backend = gui::DSMicroGuiBackend::new(video_context);
        let mut frontend = gui::AppData::new();
        FS_NAND.mount(core::ffi::CStr::from_bytes_with_nul_unchecked(b"nand:\0"));

        FS_SD.mount(core::ffi::CStr::from_bytes_with_nul_unchecked(b"sd:\0"));


        if (0x4000130 as *const u16).read_volatile() & 3 != 3 {
            frontend.autoboot();
        }

        frontend.play_startup_music();

        micro_imgui::run(backend, (), |mut f, _| {
            frontend.update(&mut f);
        });
    }
}
static mut SDMMC_DRIVER: SDMMCDriver = SDMMCDriver {
    nand_controller: None,
    sd_controller: None,
};

pub fn is_bootable(str: &[u8]) -> bool {
    let len = str.len() - 4;
    let Some(extension) = str.get(len..) else {
        return false;
    };
    extension == b".APP" || extension == b".NDS" || extension == b".DSI"
}
pub fn is_music_module(str: &[u8]) -> bool {
    let len = str.len() - 4;
    let Some(extension_range) = str.get(len..) else {
        return false;
    };
    extension_range == b".MOD"
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
        stack_irq = const DSI_WRAM_START + 0x2000,
        stack_svc = const DSI_WRAM_START + 0x4000,
        stack_sys = const DSI_WRAM_START + 0x6000,
        main = sym main, // Link the `main` symbol
        options(noreturn) // No return possible from this function
    );
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
fn write_sd_card(buffer: *mut [reboot_lib::StorageSector], start_sector: u32) -> Result<(), u32> {
    unsafe {
        flush_mmc();
        reboot_lib::arm9_set_buffer(buffer)?;
        reboot_lib::arm9_write_sd_sector(start_sector)?;
        flush_mmc();
        flush_mmc();
    }
    Ok(())
}
pub fn read_controller() -> (Buttons, u8, u8) {
    unsafe { reboot_lib::arm9_send_controller_read() }
}
fn _read_firmware(buffer: *mut [reboot_lib::StorageSector], start_offset: u32) {
    unsafe {
        reboot_lib::arm9_set_buffer(buffer);
        reboot_lib::arm9_read_firmware(start_offset);
    }
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("mov r11, r11");
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b0111110100000000);
        core::ptr::write_volatile(0x5000400 as *mut u16, 0b0111110100000000);

        let mut video_context = reboot_lib::VideoHardwareHandle::new();

        video_context.next_frame();
        gui::VideoTextPass::new(
            &mut video_context,
            micro_imgui::Rect::from_min_size(Vec2::ZERO, Vec2::new(255, 191)),
        )
        .text_pass(|text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.set_position(60, 80);
            text_pass.layout_str("HARD CRASH!!! ", 8);
        });
        video_context.next_frame();
        gui::VideoTextPass::new(
            &mut video_context,
            micro_imgui::Rect::from_min_size(Vec2::ZERO, Vec2::new(255, 191)),
        )
        .text_pass(|text_pass| {
            
            text_pass.set_color(0x7FFF);
            text_pass.set_position(60, 80);
            text_pass.layout_str("Software version: ", 8);
            text_pass.layout_str(env!("CARGO_PKG_VERSION"), 8);

            text_pass.set_position(7, 24);
            text_pass.layout_str("The console crashed!", 16);
            text_pass.set_position(0, 44);
            text_pass.layout_str(" It's safe to restart the console and try  again. For support please visit the DSi   hacking server on discord", 8);
            
            text_pass.set_position(0, 120);
            text_pass.set_color(0x7766);
            text_pass.layout_str("Error Message:", 8);
            text_pass.next_line();
            
            print_msg(info, text_pass);
            
        });
        video_context.next_frame();
        loop {
            (0x400_0208 as *mut u32).write_volatile(0);
        }
    }
}
pub struct PanicFmt{ 
    base: *mut u8, 
    len: usize, 
    cap: usize,
}
impl PanicFmt {
    pub fn new(ptr: *mut u8, size: usize) -> Self {
        Self { base: ptr, len: 0, cap: size }
    }
}
impl core::fmt::Write for PanicFmt {
    fn write_str(&mut self, arg: &str) -> Result<(), core::fmt::Error> {
        for byte in arg.as_bytes() {
            if self.len < self.cap {
                unsafe { self.base.add(self.len).write(*byte); };
                self.len += 1;
            } else { break;}
        }
        Ok(())
    }
}
impl PanicFmt {
    pub fn as_str(&self) -> &str {
        //SAFETY: the only way to modify the fmt is by writing a str into it. Therefore it is valid utf8.
        unsafe { str::from_raw_parts(self.base as *const u8, self.len) }
    }
}
unsafe fn print_msg(info: &core::panic::PanicInfo, text_pass: &mut TextLayoutHandle) {
     
    let mut buf = PanicFmt::new(0x20F_0000 as *mut u8, 0x1000);
    use core::fmt::Write;
    write!(&mut buf, "{}",info.message());
    text_pass.layout_str(buf.as_str(),8);
    if let Some(loc) = info.location(){
        use core::fmt::Write;
        text_pass.next_line();
        text_pass.next_line();
        text_pass.layout_str("Error location:",8);
        text_pass.next_line();
        
        let mut buf =  PanicFmt::new(0x20F_1000 as *mut u8, 0x1000);//if 
        write!(buf, "{loc}");
        text_pass.layout_str(buf.as_str(), 8);
    };
    
}
#[inline]
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
