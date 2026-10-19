use reboot_lib::{
    DisplayControl, MatrixMode, PolygonAttributes, VRAMCtrl, VideoHardwareHandle,
    VideoPowerControl, Viewport, VIDEO_HARDWARE,
};

use crate::graphics::font::init_font;

unsafe fn init_3d_hardware(video_context: &mut VideoHardwareHandle) {
    //setup 3d hardware
    VIDEO_HARDWARE
        .power_control
        .write(VideoPowerControl::all() ^ VideoPowerControl::ENGINE_A_ON_TOP);
    VIDEO_HARDWARE
        .vram_control_bank_a
        .write(VRAMCtrl::ENABLE | VRAMCtrl::MST_3); //map VRAM BANK A
    VIDEO_HARDWARE
        .vram_control_bank_f
        .write(VRAMCtrl::ENABLE | VRAMCtrl::MST_3); //map VRAM BANK F

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
    /*
    VIDEO_HARDWARE
        .geometry_commands
        .material_texture_attributes
        .write((7 << 20) | (2 << 26) | (1 << 29) | 0x3000); //bind font texture
    */
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

pub unsafe fn init_graphics() -> VideoHardwareHandle {
    //enable VRAM bank A
    VIDEO_HARDWARE
        .vram_control_bank_a
        .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);
    VIDEO_HARDWARE
        .vram_control_bank_f
        .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);

    //enable the 2D engine A, with no backgrounds on.
    VIDEO_HARDWARE
        .engine_a_ctrl
        .write(DisplayControl::BG_MODE_0 | DisplayControl::ENABLE_BG_0);

    // Set background 3 on the sub engine to cover the whole screen and be flipped vertically to prepare for wallpaper
    // (Note: this is done since BMP bitmaps are "vertically flipped", starting from the bottom left and ending on the top right)
    VIDEO_HARDWARE.engine_b_bgctrl[3].write((1 << 14) | (1 << 7) | (1 << 2));
    VIDEO_HARDWARE.engine_b_bgscrl[6].write(0);
    VIDEO_HARDWARE.engine_b_bgscrl[7].write(0);
    VIDEO_HARDWARE.engine_b_bg3_ref[0].write(0);
    VIDEO_HARDWARE.engine_b_bg3_ref[1].write((191) << 8);

    VIDEO_HARDWARE.engine_b_bg3_scale[0].write(256);
    VIDEO_HARDWARE.engine_b_bg3_scale[1].write(0);
    VIDEO_HARDWARE.engine_b_bg3_scale[2].write(0);
    VIDEO_HARDWARE.engine_b_bg3_scale[3].write(0xFF00);

    //copy font to vram

    init_font();

    let mut video_context = reboot_lib::VideoHardwareHandle::new();
    init_3d_hardware(&mut video_context);
    video_context.next_frame();
    video_context
}
