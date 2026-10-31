// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

#![no_main]
#![no_std]
#![feature(str_from_raw_parts)]
#![feature(asm_const_ptr)]

extern crate alloc;

pub mod arm7_exploit;
pub mod configuration;
pub mod fat;
pub mod graphics;
pub mod music;
pub mod resources;
pub mod twl_wifi;

mod boot;
mod files;
mod gui;
mod nand;
mod screen_fade;

pub use files::{filetype, get_extension, truncate_name, FileEntry, FileType};
pub use micro_imgui_ds::SCREEN_RECT;
pub use screen_fade::{irq_screen_fade, Fader};

use crate::boot::read_all;
use crate::configuration::{Config, Theme};
use crate::fat::driver::SDMMCDriver;
use crate::graphics::bmp::AsyncBMPLoader;
use crate::gui::{AppData, GlobalData};
use alloc::vec;
use alloc::{boxed::Box, string::String};
use common::blowfish::BFCTX;
use core::str;
use fatfs_embedded::fatfs::{FileOptions, RawFileSystem};
use graphics::font::load_default_font;
use micro_imgui_ds::read_controller;
use reboot_lib::autoboot_info::{UnlaunchBootFlags, BOOT_INFO};
use reboot_lib::{fatfs_embedded, LCDStatus};

use reboot_lib::{bytemuck, Interrupt, VRAMCtrl, IPC_FIFO_HARDWARE};
use reboot_lib::{Buttons, VideoPowerControl, VIDEO_HARDWARE};

// Where the NWRAM bank for this code start
const DSI_WRAM_START: usize = 0x037C0000;
// Where the TMD code is actually placed
const BINARY_START: usize = 0x037DF27C;
// Where to put the app area (arbitrary, memory before this is used as stack space)
const APP_AREA_START: usize = DSI_WRAM_START + 0xC000;
// How long the app area may theorhetically be as to not collide with the TMD
const APP_AREA_LEN: usize = BINARY_START - APP_AREA_START;

const BACKGROUND_COLOR: u16 = 0b0_00100_00100_00100;

extern "C" {
    static mut _itcm_addr: u32;
    static _itcm_len_words: u8;
    static _aux_off: u8;
    static _aux_len: u8;
}
pub struct FileSystems {
    pub nand_fs: RawFileSystem,
    pub sdmc_fs: RawFileSystem,
}
pub struct AppArea {
    sdmmc_driver: core::mem::MaybeUninit<SDMMCDriver>,
    app_data: core::mem::MaybeUninit<AppData>,
    filesystems: FileSystems,
    wav_counter: reboot_lib::volatile_register::RW<u32>,
    path_buffer: [u8; 256],
}
reboot_lib::const_assert!(core::mem::size_of::<AppArea>() < APP_AREA_LEN);

pub unsafe fn steal_main_mem() {
    reboot_lib::ALLOCATOR.init();
}

unsafe fn early_crash(message: &str) -> ! {
    load_default_font();
    set_bright(0 | (1 << 14));
    let mut video_context = graphics::init_graphics();
    micro_imgui_ds::gui::VideoTextPass::new(&mut video_context, SCREEN_RECT).text_pass(
        |text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.next_line();
            text_pass.layout_str("oh no!", 16);
            text_pass.next_line();
            text_pass.next_line();
            text_pass.layout_str(
                "If you can see this screen then astronaut failed badly.",
                8,
            );
            text_pass.next_line();
            text_pass.next_line();
            text_pass.layout_str("For support, go to the DSi hacking server on Discord or the dsi.cfw.guide website.", 8);
            text_pass.next_line();
            text_pass.next_line();
            text_pass.layout_str(
                "Alternatively, try to reach me via email: viktor@koda.re",
                8,
            );
            text_pass.next_line();
            text_pass.next_line();
            text_pass.layout_str("Error message: ", 8);
            text_pass.layout_str(message, 8);
        },
    );
    video_context.next_frame();
    loop {}
}

/// Loads the code thats within the ".text_aux" segment into memory
unsafe fn load_aux_segment(data: &mut GlobalData) -> bool {
    let Ok(mut file) = fatfs_embedded::open(&mut data.our_path, FileOptions::Read) else {
        return false;
    };
    if fatfs_embedded::seek(&mut file, core::ptr::addr_of!(_aux_off) as u32).is_err() {
        return false;
    }
    let remaining_size = fatfs_embedded::size(&mut file) - file.fptr;
    let mut binary_copy = vec![0u32; (remaining_size >> 2) as usize];
    {
        let as_bytes: &mut [u8] = bytemuck::must_cast_slice_mut(&mut binary_copy);
        if read_all(as_bytes, &mut file).is_err() {
            return false;
        }
    }
    let mut word_iter = binary_copy.into_iter();
    VIDEO_HARDWARE
        .vram_control_bank_e
        .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);
    for i in 0..0x4000 {
        (0x6880000 as *mut u32)
            .add(i)
            .write(word_iter.next().unwrap_or(0));
    }
    true
}
#[link_section = ".text_itcm"]
unsafe fn set_bright(factor: u16) {
    VIDEO_HARDWARE.master_brightness.write(factor);
    VIDEO_HARDWARE.disp_b_master_bright.write(factor);
}
unsafe fn main() {
    unsafe {
        // Print debug string visible on emulators
        reboot_lib::nocash_write("> Welcome to astronaut!\n");
        // Create App area, used as "God struct"
        let app_area = &mut *(APP_AREA_START as *mut AppArea);

        // Turn on the screens (won't be visible until ARM7 turns on the backlight)
        VIDEO_HARDWARE
            .power_control
            .write(VideoPowerControl::all() ^ VideoPowerControl::ENGINE_A_ON_TOP);

        (0x4000204 as *mut u16).write_volatile((1 << 15) | (1 << 13));

        // Set the Screen to all white
        set_bright(16 | (1 << 14));
        // Set the backgrounds to a default color
        graphics::set_background(BACKGROUND_COLOR);

        // Initialize IPC
        IPC_FIFO_HARDWARE.enable();
        IPC_FIFO_HARDWARE.set_status(0);

        // Take over ARM7 Co-processor with another exploit
        arm7_exploit::takeover_arm7();

        // Steal the 16MB of main mem to use most of it as a heap
        steal_main_mem();

        // Check in with the ARM7 to make sure it's alive after the exploit
        let mut timeout_counter = 0;
        while IPC_FIFO_HARDWARE.read_status() != 1 {
            timeout_counter += 1;
            if timeout_counter > 0x800000 {
                early_crash("arm7 exploit failed");
            }
        }
        IPC_FIFO_HARDWARE.set_status(1);
        while IPC_FIFO_HARDWARE.read_status() != 0 {
            timeout_counter += 1;
            if timeout_counter > 0x800000 {
                early_crash("arm7 exploit failed");
            }
        }

        // ARM7 is alive! make sure to let it know we know.
        IPC_FIFO_HARDWARE.set_status(0);

        core::ptr::write_volatile(0x4000304 as *mut u32, 0b1000001111);
        reboot_lib::interupts::init_interrupts();

        IPC_FIFO_HARDWARE.enable_irqs();

        reboot_lib::set_interrupt_function(Interrupt::VBlank, irq_screen_fade);
        reboot_lib::enable_interrupt(Interrupt::IPCNonEmpty);
        music::init();

        VIDEO_HARDWARE.lcd_status.write(LCDStatus::VBLANK_IRQ);

        let sdmmc_driver = app_area.sdmmc_driver.write(SDMMCDriver::new());
        fatfs_embedded::fatfs::diskio::install(sdmmc_driver);

        // Mount NAND (eMMC)
        let _ = app_area.filesystems.nand_fs.mount(c"nand:");

        // Mount SD/MMC Card Slot
        let _ = app_area.filesystems.sdmc_fs.mount(c"sdmc:");

        // Find the TMD we were launched from (if any, string left over by stage 2)
        let tmd_path = core::ffi::CStr::from_ptr(0x02FE34C4 as *const _)
            .to_str()
            .map(|i| String::from("nand:/") + i)
            .unwrap_or(String::new());

        // Initialize the state of ourselves
        let app_data = {
            let ptr = app_area.app_data.as_mut_ptr();
            (&raw mut (*ptr).current_ui).write(Box::new(gui::MainMenu));
            (&raw mut (*ptr).global_data.our_path).write(tmd_path);
            (&raw mut (*ptr).global_data.loading_mod_file).write(music::MusicPlaying::None);
            (&raw mut (*ptr).global_data.config).write(Config::default());
            (&raw mut (*ptr).global_data.theme).write(Theme::DEFAULT);
            (&raw mut (*ptr).global_data.bmp_loader).write(AsyncBMPLoader::new());
            (&raw mut (*ptr).global_data.queued_bmp).write(None);

            // Copy the blowfish key present in ITCM from stage 2
            (&raw mut (*ptr).global_data.blowfish).write((*(0x1FFC894 as *const BFCTX)).clone());

            (&raw mut (*ptr).global_data.safe_mode).write(false);

            (&mut (*ptr).global_data.fader).set_state(16);

            app_area.app_data.assume_init_mut()
        };
        reboot_lib::enable_interrupt(Interrupt::VBlank);

        // Figure out what to start
        let (buttons, _, _) = read_controller();
        let force_menu = buttons == (Buttons::BUTTON_A | Buttons::BUTTON_B);
        app_data.global_data.config.load(buttons);

        // This is the point where astronaut should try to load something else
        if !force_menu {
            // Chainload by other homebrew
            if let Some(params) = BOOT_INFO.unlaunch.parameters() {
                if params.flags.contains(UnlaunchBootFlags::BOOT) {
                    let mut file_path = params.parse_path();
                    if let Ok(mut file) = fatfs_embedded::open(&mut file_path, FileOptions::Read) {
                        app_data.global_data.fader.set_target(16);
                        boot::boot_app(&mut file, &mut file_path, &mut app_data.global_data);
                    }
                }
            }
            // Load default app based on config
            app_data.autoboot();
        }

        // Load additional code segment required for the GUI
        if !load_aux_segment(&mut app_data.global_data) {
            //Enable safe mode (fill code area with infinite loops instead)
            VIDEO_HARDWARE
                .vram_control_bank_e
                .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);
            for i in 0..0x4000 {
                (0x6880000 as *mut u32).add(i).write(0xe7fee7fe);
            }
            app_data.global_data.safe_mode = true;
        }
        gui::load_gui(app_data, buttons);
    }
}

#[no_mangle]
#[link_section = ".text_entry"]
#[cfg(target_arch = "arm")]
#[instruction_set(arm::a32)]
pub unsafe extern "C" fn _start() {
    core::arch::asm!(
        // Dev console shifter, fixes dev console compat (entrypoint for dev consoles).
        "13:",
        "mov r0, #0x04000000",
        "str r0, [r0, #0x208]",
        "ldr r1, =10f", //dest
        "add r0, r1, 0x80", //source
        "add r2, r1, 0x14000", //limit
        "12:",
        "ldmia r0!, {{r3-lr}}",
        "stmia r1!, {{r3-lr}}",
        "cmp r1, r2",
        "blt 12b",

        // Dev console trap, catches dev consoles.
        ".word 0, 0, 0, 0, 0, 0, 0, 0",
        ".word 0, 0, 0, 0, 0, 0, 0, 0",
        "b 13b",
        ".word 0, 0, 0, 0, 0, 0, 0, 0",
        ".word 0, 0, 0, 0, 0",
        // turn off interrupts via the IME register (entrypoint for retail consoles)
        "mov r0, #0x04000000",
        "str r0, [r0, #0x208]",

        "10:",
        // load start of stack(s)
        "mov r0, #0x12",
        "msr cpsr, r0",
        "ldr sp, ={stack_irq}",

        "mov r0, #0x13",
        "msr cpsr, r0",
        "ldr sp, ={stack_svc}",

        "mov r0, #0x1F",
        "msr cpsr, r0",
        "ldr sp, ={stack_sys}",

        // Load ITCM segment
        "ldr r0, ={itcm}",
        "mov r1, #0x01000000",
        "mov r2, #0x1000",

        "10: subs r2, 32",
        "ldmia r0!, {{r3-r10}}",
        "stmia r1!, {{r3-r10}}",
        "bne 10b",

        // Load Exception vector
        "ldr r0, =9f",
        "ldr r1, =0x2FFFD9C",
        "str r0, [r1]",

        // Call the main function
        "bl {main}",

        // Halt the CPU after main returns (if it does)
        "11: b 11b", // Infinite loop

        // Exception manifold
        "1: mov r0, #1","bl {exception}",
        "2: mov r0, #2","bl {exception}",
        "3: mov r0, #3","bl {exception}",
        "4: mov r0, #4","bl {exception}",
        "5: mov r0, #5","bl {exception}",
        "6: mov r0, #6","bl {exception}",
        "7: mov r0, #7","bl {exception}",
        "8: mov r0, #8","bl {exception}",
        // Exception vector
        "9:",
        "bl 1b",
        "bl 2b",
        "bl 3b",
        "bl 4b",
        "bl 5b",
        "bl 6b",
        "bl 7b",
        "bl 8b",

        stack_irq = const DSI_WRAM_START + 0x2000, // 8KB IRQ Stack
        stack_svc = const DSI_WRAM_START + 0x4000, // 8KB SVC Stack
        stack_sys = const DSI_WRAM_START + 0x8000, // 16KB System/User Stack
        itcm = const ITCM,
        main = sym main, // Link the `main` symbol
        exception = sym exception,
        options(noreturn) // No return possible from this function
    );
    const ITCM: *const u32 = core::ptr::addr_of!(_itcm_addr);
}
#[no_mangle]
unsafe extern "C" fn exception(kind: u32) {
    let a = match kind {
        1 => "RESET",
        2 => "UNDEFINED INSTRUCTION",
        3 => "SVC CALL",
        4 => "PREFETCH ABORT",
        5 => "DATA ABORT",
        6 => "RESERVED",
        7 => "IRQ",
        8 => "FIQ",
        _ => "UNKNOWN",
    };
    panic!("EXCEPTION {a}");
}

#[no_mangle]
#[cfg(not(target_arch = "arm"))]
pub unsafe extern "C" fn _start() {
    main();
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use micro_imgui_ds::gui;
    use micro_imgui_ds::micro_imgui;
    use micro_imgui_ds::micro_imgui::Vec2;
    unsafe {
        set_bright(0 | (1 << 14));
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
pub struct PanicFmt {
    base: *mut u8,
    len: usize,
    cap: usize,
}
impl PanicFmt {
    pub fn new(ptr: *mut u8, size: usize) -> Self {
        Self {
            base: ptr,
            len: 0,
            cap: size,
        }
    }
}
impl core::fmt::Write for PanicFmt {
    fn write_str(&mut self, arg: &str) -> Result<(), core::fmt::Error> {
        for byte in arg.as_bytes() {
            if self.len < self.cap {
                unsafe {
                    self.base.add(self.len).write(*byte);
                };
                self.len += 1;
            } else {
                break;
            }
        }
        Ok(())
    }
}
impl PanicFmt {
    pub fn as_str(&self) -> &str {
        //SAFETY: the only way to modify the fmt is by writing a str into it. Therefore it is valid utf8 as long as the str's pushed onto it are.
        unsafe { str::from_raw_parts(self.base as *const u8, self.len) }
    }
}

#[cfg(target_arch = "arm")]
unsafe fn print_msg(
    info: &core::panic::PanicInfo,
    text_pass: &mut micro_imgui_ds::gui::TextLayoutHandle,
) {
    let mut buf = PanicFmt::new(0x20F_0000 as *mut u8, 0x1000);
    use core::fmt::Write;
    let _ = write!(&mut buf, "{}", info.message());

    text_pass.layout_str(buf.as_str(), 8);
    if let Some(loc) = info.location() {
        use core::fmt::Write;
        text_pass.next_line();
        text_pass.next_line();
        text_pass.layout_str("Error location:", 8);
        text_pass.next_line();

        let mut buf = PanicFmt::new(0x20F_1000 as *mut u8, 0x1000);
        let _ = write!(buf, "{loc}");
        text_pass.layout_str(buf.as_str(), 8);
    };
}
