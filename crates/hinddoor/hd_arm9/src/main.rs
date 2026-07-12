#![no_main]
#![no_std]
#![feature(ptr_metadata)]
#![feature(str_from_utf16_endian)]
#![feature(str_from_raw_parts)]
#![feature(str_split_remainder)]

const BOOTSTRAP_BINARY: &[u8] = include_bytes!("./bootstrap.bin");
pub mod bmp;
pub struct FileSystems {
    pub nand_fs: RawFileSystem,
    pub sdmc_fs: RawFileSystem,
}
pub struct AppArea {
    sdmmc_driver: core::mem::MaybeUninit<SDMMCDriver>,
    app_data: core::mem::MaybeUninit<AppData>,
    filesystems: FileSystems,
    fader: Fader,
    wav_counter: reboot_lib::volatile_register::RW<u32>,
    path_buffer: [u8; 256],
}
pub struct Fader {
    current: reboot_lib::volatile_register::RW<i8>,
    target: reboot_lib::volatile_register::RW<i8>,
}
reboot_lib::const_assert!(core::mem::size_of::<AppArea>() < APP_AREA_LEN);

use alloc::{boxed::Box, string::String, vec::Vec};
use common::blowfish::BFCTX;
use core::str;
use fatfs_embedded::fatfs::{FileOptions, RawFileSystem};
use micro_imgui_ds::read_controller;
use reboot_lib::autoboot_info::{UnlaunchBootFlags, BOOT_INFO};
use reboot_lib::timers::TimerControl;

use micro_imgui_ds::micro_imgui::Color;
use reboot_lib::music_modules::mods::MODHeader;
use reboot_lib::{
    flush_mmc, Interrupt, VRAMCtrl, VideoHardwareHandle, ENGINE_A_PALETTES, ENGINE_B_PALETTES,
    IPC_FIFO_HARDWARE,
};
use reboot_lib::{
    Buttons, DisplayControl, MatrixMode, PolygonAttributes, VideoPowerControl, Viewport,
    VIDEO_HARDWARE,
};

use crate::boot::read_all;
use crate::fat::driver::SDMMCDriver;
use crate::gui::AppData;

extern crate alloc;

mod boot;
pub mod configuration;
pub mod fat;
mod gui;
mod mbr;
mod nand;
pub mod new_takeover;
#[repr(C)]
pub struct NandAutobootEntry {
    category: u16,
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

pub unsafe fn steal_main_mem() {
    reboot_lib::ALLOCATOR.init();
}
#[inline(always)]
pub unsafe fn unlaunch_breakpoint() {
    #[cfg(target_arch = "arm")]
    core::arch::asm!("mov r11, r11");
}

#[instruction_set(arm::a32)]
#[cfg(target_arch = "arm")]
unsafe fn load_default_font() {
    const FONT_FILE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/font_compressed.bin"));
    for i in 0..FONT_FILE.len() {
        core::ptr::write_volatile((0x2FF2000 as *mut u8).add(i), FONT_FILE[i]);
    }
    core::arch::asm!(
        "SWI 0x110000",
        in("r0") 0x2FF2000,
        in("r1") 0x2FF1000,
        lateout("r0") _,
        lateout("r1") _,
        out("r2") _,
        out("r3") _,
    );
}
#[cfg(not(target_arch = "arm"))]
unsafe fn load_default_font() {
    panic!()
}
#[instruction_set(arm::a32)]
#[cfg(target_arch = "arm")]
unsafe fn init_font() {
    transfer_font_to_vram();
}
unsafe fn transfer_font_to_vram() {
    for i in 0..0x200 {
        let reg = core::ptr::read_volatile((0x2FF_1000 as *const u32).add(i));
        core::ptr::write_volatile((0x6818000 as *mut u32).add(i), reg);
    }

    core::ptr::write_volatile(
        0x06880000 as *mut u32,
        core::ptr::read_volatile(0x2FF_1800 as *const u32),
    );
    core::ptr::write_volatile(
        0x06880004 as *mut u32,
        core::ptr::read_volatile(0x2FF_1804 as *const u32),
    );
}
#[cfg(not(target_arch = "arm"))]
unsafe fn init_font() {
    panic!()
}

unsafe fn init_3d_hardware(video_context: &mut VideoHardwareHandle) {
    //setup 3d hardware
    VIDEO_HARDWARE
        .power_control
        .write(VideoPowerControl::all() ^ VideoPowerControl::ENGINE_A_ON_TOP);
    VIDEO_HARDWARE
        .vram_control_bank_a
        .write(VRAMCtrl::ENABLE | VRAMCtrl::MST_3); //map VRAM BANK A
    VIDEO_HARDWARE
        .vram_control_bank_e
        .write(VRAMCtrl::ENABLE | VRAMCtrl::MST_3); //map VRAM BANK E
    VIDEO_HARDWARE
        .engine_a_ctrl
        .write(DisplayControl::BG_MODE_0 | DisplayControl::ENABLE_3D | DisplayControl::ENABLE_BG_0);
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
        .write((7 << 20) | (2 << 26) | (1 << 29) | 0x3000); //bind font texture
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

const COLOR_BOOTABLE: Color = Color::new(100, 200, 100);
const COLOR_MUSIC: Color = Color::new(100, 100, 200);
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum FileType {
    Dir,
    Rom,
    Mod,
    Wav,
    Bmp,
    Ini,
    None,
}
pub const ASSOCIATION_LIST: &[(&[u8], FileType)] = &[
    (b".WAV", FileType::Wav),
    (b".MOD", FileType::Mod),
    (b".INI", FileType::Ini),
    (b".BMP", FileType::Bmp),
    (b".NDS", FileType::Rom),
    (b".DSI", FileType::Rom),
    (b".APP", FileType::Rom),
];
pub fn filetype(extension: &[u8]) -> FileType {
    ASSOCIATION_LIST
        .iter()
        .filter_map(|(t, i)| extension.ends_with(t).then_some(i))
        .next()
        .copied()
        .unwrap_or(FileType::None)
}

#[derive(Clone)]
pub struct FileEntry {
    pub file_name: String,
    pub display_name: String,
    pub kind: FileType,
}
pub fn populate_fs_vec(folder: &mut fatfs_embedded::fatfs::Directory) -> Vec<FileEntry> {
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
                    FileType::Dir
                } else {
                    let s_name = core::ffi::CStr::from_ptr(file.altname.as_ptr()).to_bytes();
                    let s_name = if s_name.is_empty() {
                        name.as_bytes()
                    } else {
                        s_name
                    };
                    filetype(s_name)
                };
                let fname = name.clone();
                if name.len() > 35 {
                    let mut boundary = 32;
                    while !name.is_char_boundary(boundary) {
                        boundary += 1;
                    }
                    name.truncate(boundary);
                    name.push_str("...");
                }
                vec.push(FileEntry {
                    display_name: name,
                    file_name: fname,
                    kind: color,
                })
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
            if under.kind > temp.kind {
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

pub use micro_imgui_ds::SCREEN_RECT;

unsafe fn arm7_crash() -> ! {
    load_default_font();
    set_bright(0 | (1 << 14));
    let mut video_context = init_graphics();
    micro_imgui_ds::gui::VideoTextPass::new(&mut video_context, SCREEN_RECT).text_pass(
        |text_pass| {
            text_pass.set_color(0x7FFF);
            text_pass.next_line();
            text_pass.layout_str("oh no!", 16);
            text_pass.next_line();
            text_pass.next_line();
            text_pass.layout_str(
                "If you can see this screen then something has gone wrong.",
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
        },
    );
    video_context.next_frame();
    loop {}
}
unsafe fn init_graphics() -> VideoHardwareHandle {
    //enable VRAM bank A
    VIDEO_HARDWARE
        .vram_control_bank_a
        .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);
    VIDEO_HARDWARE
        .vram_control_bank_e
        .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);

    //enable the 2D engine A, with no backgrounds on.
    VIDEO_HARDWARE
        .engine_a_ctrl
        .write(DisplayControl::BG_MODE_0 | DisplayControl::ENABLE_BG_0);

    // Set background 3 on the sub engine to cover the whole screen and be flipped vertically to prepare for wallpaper
    // (Note: this is done since BMP bitmaps are "vertically flipped", starting from the bottom left and ending on the top right)
    VIDEO_HARDWARE.disp_b_bgctrl[3].write((1 << 14) | (1 << 7) | (1 << 2));
    VIDEO_HARDWARE.disp_b_bgscrl[6].write(0);
    VIDEO_HARDWARE.disp_b_bgscrl[7].write(0);
    VIDEO_HARDWARE.disp_b_bg3_ref[0].write(0);
    VIDEO_HARDWARE.disp_b_bg3_ref[1].write((191) << 8);

    VIDEO_HARDWARE.disp_b_bg3_scale[0].write(256);
    VIDEO_HARDWARE.disp_b_bg3_scale[1].write(0);
    VIDEO_HARDWARE.disp_b_bg3_scale[2].write(0);
    VIDEO_HARDWARE.disp_b_bg3_scale[3].write(0xFF00);
    VIDEO_HARDWARE
        .disp_b_control
        .write(DisplayControl::BG_MODE_5);

    //copy font to vram

    init_font();

    let mut video_context = reboot_lib::VideoHardwareHandle::new();
    init_3d_hardware(&mut video_context);
    video_context.next_frame();
    video_context
}

unsafe fn find_wifi_firmware() -> Option<String> {
    const CONTENT_FOLDER: &str = "nand:/title/0003000F/484E4341/content/";
    let app_version = {
        let mut firmware_tmd = fatfs_embedded::open(
            &mut alloc::format!("{CONTENT_FOLDER}title.tmd"),
            FileOptions::Read,
        )
        .ok()?;
        let mut app_version = [0u8; 4];
        fatfs_embedded::seek(&mut firmware_tmd, 0x1E4).ok()?;
        read_all(&mut app_version, &mut firmware_tmd).ok()?;
        u32::from_be_bytes(app_version)
    };
    Some(alloc::format!("{CONTENT_FOLDER}{app_version:08x?}.app"))
}
unsafe fn load_wifi_firmware() -> u32 {
    let mut ret = 0xDEADBEEF;
    let Some(mut firmware_path) = find_wifi_firmware() else {
        return ret;
    };
    let Ok(mut firmware) = fatfs_embedded::open(&mut firmware_path, FileOptions::Read) else {
        return ret;
    };
    let size = fatfs_embedded::size(&mut firmware);
    let layout = core::alloc::Layout::from_size_align_unchecked(size as usize, 4);
    let firmware_ptr = alloc::alloc::alloc(layout);
    let mut firmware_buffer = core::slice::from_raw_parts_mut(firmware_ptr, size as usize);
    if read_all(&mut firmware_buffer, &mut firmware).is_ok() {
        ret = match reboot_lib::arm9_init_nwifi(firmware_buffer) {
            Ok(_) => 0,
            Err(e) => e.get(),
        };
    }
    alloc::alloc::dealloc(firmware_ptr, layout);
    ret
}
unsafe fn fade_out() {
    let area = &mut (*(APP_AREA_START as *mut AppArea)).fader;
    let read = area.current.read();
    let target = area.target.read();
    let new = match read.cmp(&target) {
        core::cmp::Ordering::Less => (read + 3).min(target),
        core::cmp::Ordering::Equal => return,
        core::cmp::Ordering::Greater => (read - 2).max(target),
    };
    area.current.write(new);
    set_bright(new as u16 | (1 << 14));
}
unsafe fn set_bright(factor: u16) {
    VIDEO_HARDWARE.master_brightness.write(factor);
    VIDEO_HARDWARE.disp_b_master_bright.write(factor);
}
const BACKGROUND_COLOR: u16 = 0b0_00100_00100_00100;

unsafe fn uptick_wav() {
    (*(APP_AREA_START as *mut AppArea))
        .wav_counter
        .modify(|i| i + 1);
}
unsafe fn main() {
    unsafe {
        reboot_lib::nocash_write("> Welcome to vlaunch!\n");
        let app_area = &mut *(APP_AREA_START as *mut AppArea);

        (&raw mut (*app_area.app_data.as_mut_ptr()).global_data.blowfish)
            .write((*(0x1FFC894 as *const BFCTX)).clone());
        VIDEO_HARDWARE
            .power_control
            .write(VideoPowerControl::all() ^ VideoPowerControl::ENGINE_A_ON_TOP);

        (0x4000204 as *mut u16).write_volatile((1 << 15) | (1 << 13));

        set_bright(16 | (1 << 14));
        set_background(BACKGROUND_COLOR);

        IPC_FIFO_HARDWARE.enable();
        IPC_FIFO_HARDWARE.set_status(0);

        new_takeover::takeover_arm7();

        steal_main_mem();

        // Check in with the ARM7 to make sure it's alive
        let mut timeout_counter = 0;
        while IPC_FIFO_HARDWARE.read_status() != 1 {
            timeout_counter += 1;
            if timeout_counter > 0x800000 {
                arm7_crash();
            }
        }
        IPC_FIFO_HARDWARE.set_status(1);
        while IPC_FIFO_HARDWARE.read_status() != 0 {
            timeout_counter += 1;
            if timeout_counter > 0x800000 {
                arm7_crash();
            }
        }
        // ARM7 is alive! make sure to let it know.
        IPC_FIFO_HARDWARE.set_status(0);

        app_area.fader.target.write(16);
        app_area.fader.current.write(16);

        core::ptr::write_volatile(0x4000304 as *mut u32, 0b1000001111);
        reboot_lib::interupts::init_interrupts();

        IPC_FIFO_HARDWARE.enable_recv_irq();
        reboot_lib::timers::TIMERS[0]
            .write(reboot_lib::timers::Timer::new(0, TimerControl::empty()));
        reboot_lib::set_interrupt_function(Interrupt::VBlank, fade_out);
        reboot_lib::set_interrupt_function(Interrupt::Timer0, uptick_wav);
        reboot_lib::enable_interrupt(Interrupt::IPCNonEmpty);
        reboot_lib::enable_interrupt(Interrupt::VBlank);
        reboot_lib::enable_interrupt(Interrupt::Timer0);

        core::ptr::write_volatile(0x04000004 as *mut u16, 0xFFFF);

        app_area.sdmmc_driver.write(SDMMCDriver::new());
        let sdmmc_driver = app_area.sdmmc_driver.assume_init_mut();
        fatfs_embedded::fatfs::diskio::install(sdmmc_driver);

        let _ = app_area
            .filesystems
            .nand_fs
            .mount(core::ffi::CStr::from_bytes_with_nul_unchecked(b"nand:\0"));
        let _ = app_area
            .filesystems
            .sdmc_fs
            .mount(core::ffi::CStr::from_bytes_with_nul_unchecked(b"sdmc:\0"));

        let ptr = app_area.app_data.as_mut_ptr();
        (&raw mut (*ptr).global_data.autoboot).write(None);
        (&raw mut (*ptr).current_ui).write(Box::new(gui::MainMenu));
        (&raw mut (*ptr).global_data.loading_mod_file).write(gui::MusicPlaying::None);
        let (buttons, _, _) = read_controller();
        (&raw mut (*ptr).global_data.config).write(configuration::Config::load(buttons));

        let app_data = app_area.app_data.assume_init_mut();
        let force_menu = buttons == (Buttons::BUTTON_A | Buttons::BUTTON_B);

        if !force_menu {
            if let Some(params) = BOOT_INFO.unlaunch.parameters() {
                if params.flags.contains(UnlaunchBootFlags::BOOT) {
                    let mut file_path = params.parse_path();
                    (&raw mut app_data.global_data.autoboot)
                        .write(Some((file_path.clone(), params)));
                    if let Ok(mut file) = fatfs_embedded::open(&mut file_path, FileOptions::Read) {
                        //app_data.current_ui = CurrentUI::LoadingApp { file, file_path };
                        boot::boot_app(&mut file, &mut file_path, &mut app_data.global_data);
                    }
                }
            } else {
                app_data.autoboot();
            }
        }

        let (color, video_context) = app_data.global_data.load_theme();
        let backend = micro_imgui_ds::DSMicroGuiBackend::new(video_context, buttons);

        app_area.fader.target.write(0);

        micro_imgui_ds::micro_imgui::run(
            backend,
            color,
            app_data,
            |mut f, app_data| {
                app_data.update(&mut f);
            },
            |app_data| {
                app_data.do_background_tasks();
            },
        );
    }
}
fn show_wallpaper(bmp: crate::bmp::DecodedBMP) {
    if bmp.height() != 192 {
        return;
    }
    if bmp.width() != 256 {
        return;
    }
    let paletter = bmp.palette_table();
    let a = |chunk: &[u8]| {
        let red = paletter[((chunk[0] as usize) << 2) + 0] >> 3;
        let green = paletter[((chunk[0] as usize) << 2) + 1] >> 3;
        let blue = paletter[((chunk[0] as usize) << 2) + 2] >> 3;
        0x8000 | ((red as u16) << 10) | ((green as u16) << 5) | (blue as u16)
    };
    let b = |chunk: &[u8]| {
        let red = chunk[0] >> 3;
        let green = chunk[1] >> 3;
        let blue = chunk[2] >> 3;
        0x8000 | ((red as u16) << 10) | ((green as u16) << 5) | (blue as u16)
    };
    let pixel_iter: core::iter::Map<core::slice::ChunksExact<'_, u8>, &dyn Fn(&[u8]) -> u16> =
        match (bmp.dib.bits_per_pixel, bmp.dib.compression) {
            (16, 3) => {
                if bmp.palette_table()
                    != &[
                        00, 0x7C, 0x00, 0x00, 0xE0, 0x03, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00,
                    ]
                {
                    return;
                }
                bmp.bitmap.chunks_exact(2).map(&|chunk| {
                    let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let red = pixel & 0x1F;
                    let green = (pixel & (0x1F << 5)) >> 5;
                    let blue = (pixel & (0x1F << 10)) >> 10;
                    0x8000 | (red << 10) | (green << 5) | (blue)
                })
            }
            (8, 0) => bmp.bitmap.chunks_exact(1).map(&a),
            (32, 3) => {
                if bmp.palette_table()
                    != &[
                        00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00,
                    ]
                {
                    return;
                }
                bmp.bitmap.chunks_exact(4).map(&b)
            }
            (24, 0) => bmp.bitmap.chunks_exact(3).map(&b),
            _ => return,
        };
    unsafe {
        VIDEO_HARDWARE
            .vram_control_bank_c
            .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);
        for (i, pixel) in pixel_iter.enumerate() {
            (0x06840000 as *mut u16).add(i).write(pixel);
        }
        VIDEO_HARDWARE
            .vram_control_bank_c
            .write(VRAMCtrl::ENABLE | VRAMCtrl::MST_4);

        VIDEO_HARDWARE
            .disp_b_control
            .write(DisplayControl::BG_MODE_5 | DisplayControl::ENABLE_BG_3);
    }
}

pub unsafe fn set_background(color: u16) {
    ENGINE_A_PALETTES.bg_palettes[0].write(color);
    ENGINE_B_PALETTES.bg_palettes[0].write(color);
}

pub fn is_bootable(str: &[u8]) -> bool {
    let Some(extension) = get_extension(str) else {
        return false;
    };
    [b".APP".as_slice(), b".NDS".as_slice(), b".DSI".as_slice()].contains(&extension)
}
pub fn get_extension(str: &[u8]) -> Option<&[u8]> {
    let len = str.len().checked_sub(4)?;
    str.get(len..)
}
pub fn is_music_module(str: &[u8]) -> bool {
    let Some(extension) = get_extension(str) else {
        return false;
    };
    [b".MOD".as_slice(), b".WAV".as_slice()].contains(&extension)
}

const DSI_WRAM_START: usize = 0x037C0000;
const BINARY_START: usize = 0x037DF27C;
const APP_AREA_START: usize = DSI_WRAM_START + 0xC000;
const APP_AREA_LEN: usize = BINARY_START - APP_AREA_START;

#[no_mangle]
#[cfg(target_arch = "arm")]
#[instruction_set(arm::a32)]
pub unsafe extern "C" fn _start() {
    core::arch::asm!(
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
#[no_mangle]
#[cfg(not(target_arch = "arm"))]
pub unsafe extern "C" fn _start() {
    main();
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

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use micro_imgui_ds::gui;
    use micro_imgui_ds::micro_imgui;
    use micro_imgui_ds::micro_imgui::Vec2;
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
        //SAFETY: the only way to modify the fmt is by writing a str into it. Therefore it is valid utf8.
        unsafe { str::from_raw_parts(self.base as *const u8, self.len) }
    }
}
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

        let mut buf = PanicFmt::new(0x20F_1000 as *mut u8, 0x1000); //if
        let _ = write!(buf, "{loc}");
        text_pass.layout_str(buf.as_str(), 8);
    };
}
#[inline]
unsafe fn transmute_slice<T, U>(slice: *mut [T]) -> *mut U {
    slice as *mut T as *mut () as *mut U
}
