// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use core::marker::PhantomData;

use crate::{MemoryWrapper, VRAMCtrl};
use bitflags::bitflags;
use volatile_register::{RO, RW, WO};

///Public access to the DS video hardware registers
pub const VIDEO_HARDWARE: MemoryWrapper<VideoHardware> =
    MemoryWrapper(0x0400_0000 as *mut VideoHardware);

#[allow(const_item_mutation)]
pub const ENGINE_A_PALETTES: MemoryWrapper<PPUEngine> =
    MemoryWrapper(0x0500_0000 as *mut PPUEngine);

#[repr(C)]
pub struct PPUEngine {
    pub bg_palettes: [RW<u16>; 256],
    pub obj_palettets: [RW<u16>; 256],
}
pub const ENGINE_B_PALETTES: MemoryWrapper<PPUEngine> =
    MemoryWrapper(0x0500_0400 as *mut PPUEngine);

pub const ENGINE_A_OAM: MemoryWrapper<[u16; 512]> = MemoryWrapper(0x0700_0000 as *mut [u16; 512]);
pub const ENGINE_B_OAM: MemoryWrapper<[u16; 512]> = MemoryWrapper(0x0700_0400 as *mut [u16; 512]);

pub struct VideoHardwareHandle;
pub struct VideoHardwareInUseError;
impl VideoHardwareHandle {
    pub unsafe fn new() -> Self {
        Self
    }
    #[inline]
    pub unsafe fn next_frame(&mut self) {
        VIDEO_HARDWARE
            .geometry_commands
            .pipeline_swap_buffers
            .write(0);
    }
    #[inline]
    pub unsafe fn init_matricies(&mut self) {
        VIDEO_HARDWARE
            .geometry_commands
            .matrix_mode
            .write(MatrixMode::PROJECTION);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack

        VIDEO_HARDWARE
            .geometry_commands
            .matrix_mode
            .write(MatrixMode::POSITION);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack
        VIDEO_HARDWARE
            .geometry_commands
            .matrix_mode
            .write(MatrixMode::TEXTURE);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack
        VIDEO_HARDWARE
            .geometry_commands
            .matrix_mode
            .write(MatrixMode::VECTOR);
        VIDEO_HARDWARE.geometry_commands.matrix_identity.write(0); //loads an identity matrix into the selected stack
    }
    #[inline]
    unsafe fn begin_vertex_list(&mut self, primitive_type: VertexListType) {
        VIDEO_HARDWARE
            .geometry_commands
            .pipeline_begin_vertex_list
            .write(primitive_type);
    }
    #[inline]
    unsafe fn end_vertex_list(&mut self) {
        VIDEO_HARDWARE
            .geometry_commands
            .pipeline_end_vertex_list
            .write(0);
    }
    pub unsafe fn create_vertex_list<R, F: FnOnce(&mut VertexListHost) -> R>(
        &mut self,
        primitive_type: VertexListType,
        closure: F,
    ) -> R {
        self.begin_vertex_list(primitive_type);
        let mut host = VertexListHost(PhantomData);
        let ret = closure(&mut host);
        self.end_vertex_list();
        ret
    }
}

// VideoHardwareHandle is a ZST since it directly interacts with video hardware. why hold a pointer to
// it when we can only pretend to for the sake of leveraging rust lifetimes, without wasting memory?
pub struct VertexListHost<'a>(PhantomData<&'a mut VideoHardwareHandle>);
impl<'a> VertexListHost<'a> {
    pub fn set_vertex_color(&mut self, color: u32) {
        unsafe {
            VIDEO_HARDWARE
                .geometry_commands
                .vertex_set_color
                .write(color)
        };
    }
    pub fn set_texture(&mut self, texture: u32) {
        unsafe {
            VIDEO_HARDWARE
                .geometry_commands
                .material_texture_attributes
                .write(texture);
        }
    }
    pub unsafe fn to_owned(&mut self) -> Self {
        Self(self.0)
    }
    pub fn vertex_set_texture_coordinate(&mut self, x: i16, y: i16) {
        let x = x as u32;
        let y = (y as u32) << 16;
        unsafe {
            VIDEO_HARDWARE
                .geometry_commands
                .vertex_set_texture_coordinate
                .write(x | y)
        };
    }
    pub fn add_vertex_single(&mut self, x: i16, y: i16, z: i16) {
        let x = (x as u32) >> 6;
        let y = ((y as u32) << 4) & 0b11111111110000000000;
        let z = ((z as u32) << 14) & 0b111111111100000000000000000000;
        unsafe {
            VIDEO_HARDWARE
                .geometry_commands
                .vertex_set_coordinate_single
                .write(x | y | z);
        }
    }
    pub fn add_vertex_double(&mut self, x: i16, y: i16, z: i16) {
        let x = x as u32;
        let y = (y as u32) << 16;
        let z = z as u32;
        unsafe {
            VIDEO_HARDWARE
                .geometry_commands
                .vertex_set_coordinate_double
                .write(x | y);
            VIDEO_HARDWARE
                .geometry_commands
                .vertex_set_coordinate_double
                .write(z);
        }
    }
    pub unsafe fn add_vertex_relative_raw(&mut self, value: u32) {
        VIDEO_HARDWARE
            .geometry_commands
            .vertex_set_coordinate_relative
            .write(value);
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct VideoPowerControl: u16 {
        const ENABLE_ENGINE_A = (1<<1);
        const ENABLE_3D_RENDERING = (1<<2);
        const ENABLE_3D_GEOMETRY = (1<<3);
        const ENABLE_ENGINE_B = (1<<9);
        const ENGINE_A_ON_TOP = (1<<15);
        const ENABLE_LCDS = (1<<0);
    }
    #[derive(Clone, Copy)]
    pub struct DisplayControl: u32 {
        const BG_MODE_0 = 0x10000;
        const BG_MODE_1 = 0x10001;
        const BG_MODE_2 = 0x10002;
        const BG_MODE_3 = 0x10003;
        const BG_MODE_4 = 0x10004;
        const BG_MODE_5 = 0x10005;
        const BG_MODE_6 = 0x10006;

        const ENABLE_BG_0 = (1 << 8);
        const ENABLE_BG_1 = (1 << 9);
        const ENABLE_BG_2 = (1 << 10);
        const ENABLE_BG_3 = (1 << 11);

        /// Enable using the 3D hardware
        ///
        /// NOTE: requires that BG0 is enabled (and 3d hardware is powered on?)
        const ENABLE_3D = (1 << 3);
        const ENABLE_EXT_BG_PALETTE = (1 << 30);
        const ENABLE_EXT_OBJ_PALETTE = (1 << 31);
    }
    #[derive(Clone, Copy)]
    pub struct PolygonAttributes: u32 {
        const ENABLE_LIGHT_0 = (1 << 0);
        const ENABLE_LIGHT_1 = (1 << 1);
        const ENABLE_LIGHT_2 = (1 << 2);

        const POLYGON_MODE_MODULATION = (0 << 4);
        const POLYGON_MODE_SHADOW = (3 << 4);
        const POLYGON_MODE_DECAL = (1 << 4);
        const POLYGON_MODE_TOON = (2 << 4);

        const RENDER_BACK_SURFACE = (1 << 6);
        const RENDER_FRONT_SURFACE = (1 << 7);
        const RENDER_FAR_POLYGONS = (1<<12);
        const RENDER_SMALL_POLYGONS = (1<<13);
        const RENDER_FOG = (1<<15);

        const DEPTH_TEST_EQ = (1<<14);

        const POLYGON_ALPHA_SOLID = (31<<16);
        const POLYGON_ALPHA_WIREFRAME = (0<<16);

    }
    #[derive(Clone, Copy)]
    pub struct MatrixMode: u32 {
        const PROJECTION = 0;
        const POSITION = 1;
        const VECTOR = 2;
        const TEXTURE = 3;
    }

    #[derive(Clone, Copy)]
    pub struct VertexListType: u32 {
        const IndividualTris = 0;
        const IndividualQuads = 1;
        const StripTris = 2;
        const StripQuads = 3;
    }

    #[derive(Clone, Copy)]
    pub struct Viewport: u32 {
        const X0 = 0xFF << 0;
        const Y0 = 0xFF << 8;
        const X1 = 0xFF << 16;
        const Y2 = 0xFF << 24;
        const WHOLE_SCREEN_DEFAULT = (255 << 16) | (191 << 24);
    }
}
impl Viewport {
    pub const fn new(x0: u8, y0: u8, x1: u8, y1: u8) -> Self {
        Self::from_bits_retain(
            (x0 as u32) | ((y0 as u32) << 8) | ((x1 as u32) << 16) | ((y1 as u32) << 24),
        )
    }
}

#[repr(C)]
pub struct VideoHardware {
    pub engine_a_ctrl: RW<DisplayControl>,
    _0x4: u16,
    pub vcount: RO<u16>,
    _0x8: [u8; 0x58],
    pub display_control_3d: RW<u16>,
    _0x62: u16,
    pub display_capture: RW<u32>,
    pub display_memory: RW<u32>,
    pub master_brightness: RW<u16>,
    _0x6e: [u8; 0x1D2],
    pub vram_control_bank_a: WO<VRAMCtrl>,
    pub vram_control_bank_b: WO<VRAMCtrl>,
    pub vram_control_bank_c: WO<VRAMCtrl>,
    pub vram_control_bank_d: WO<VRAMCtrl>,
    pub vram_control_bank_e: WO<VRAMCtrl>,
    pub vram_control_bank_f: WO<VRAMCtrl>,
    pub vram_control_bank_g: WO<VRAMCtrl>,
    _0x247: u8,
    pub vram_control_bank_h: WO<VRAMCtrl>,
    pub vram_control_bank_i: WO<VRAMCtrl>,
    _unimplemented_3: [u8; 0xBA],
    pub power_control: WO<VideoPowerControl>,
    _unimplemented_4: [u8; 0x1A],
    pub rendered_lines: RO<u8>,
    _free_bus: [u8; 15],
    pub edge_colors: [WO<u16>; 8],
    pub alpha_test_ref: WO<u8>,
    _free_bus2: [u8; 15],
    pub clear_color: WO<Color>,
    _extra_pad: u16,
    pub clear_depth: WO<u16>,
    pub clear_offset: WO<u16>,
    pub fog_color: WO<u32>,
    pub fog_offset: WO<u16>,
    _free_bus3: u16,
    pub fog_table: [WO<u8>; 0x20],
    pub toon_table: [WO<u16>; 0x20],
    _free_bus_4: [u8; 0x40],
    pub geometry_commands: GeometryCommands,
    _free_bus_5: [u8; 0x34],
    pub geometry_engine_status: RW<u32>,
    pub ram_count: RO<u32>,
    _free_bus_6: [u8; 8],
    pub depth_boundary: WO<u16>,
    _free_bus_7: [u8; 0xE],
    pub pos_test_results: [RO<u32>; 4],
    pub vec_test_results: [RO<u16>; 3],
    _free_bus_8: [u8; 0xA],
    pub clip_matrix_result: [RO<u32>; 0x10],
    pub vector_matrix_result: [RO<u32>; 6],
    _0x698: [u8; 0x968],
    pub disp_b_control: RW<DisplayControl>,
    _0x1004: u32,
    pub disp_b_bgctrl: [RW<u16>; 4],
    pub disp_b_bgscrl: [RW<u16>; 8],
    pub disp_b_bg2_scale: [RW<u16>; 4],
    pub disp_b_bg2_ref: [RW<u32>; 2],
    pub disp_b_bg3_scale: [RW<u16>; 4],
    pub disp_b_bg3_ref: [RW<u32>; 2],
    pub gba_registers: [u8; 0x18],
    _0x1058: [u8; 0x14],
    pub disp_b_master_bright: RW<u16>,
}

#[derive(Clone, Copy)]
pub struct Color(u16);
impl Color {
    pub const WHITE: Self = Self::new_rgb(0xFF, 0xFF, 0xFF);
    pub const BLACK: Self = Self::new_rgb(0, 0, 0);
    pub const CONFIRM_GREEN: Self = Self(0b0000111101010100);
    pub const fn new_rgb(mut r: u8, mut g: u8, mut b: u8) -> Self {
        r >>= 3;
        g &= 0b11111000;
        b &= 0b11111000;
        Self((r as u16) | ((g as u16) << 2) | ((b as u16) << 7))
    }
}
#[repr(C)]
pub struct GeometryCommands {
    //GEOMETRY COMMAND FIFO 0x400_0400
    pub fifo: [WO<u32>; 0x10],

    //MATRIX OPERATIONS 0x400_0440
    pub matrix_mode: WO<MatrixMode>,
    pub matrix_push: WO<u32>,
    pub matrix_pop: WO<u32>,
    pub matrix_store: WO<u32>,
    pub matrix_restore: WO<u32>,
    pub matrix_identity: WO<u32>,
    pub matrix_load_4x4: WO<u32>,
    pub matrix_load_4x3: WO<u32>,
    pub matrix_mult_4x4: WO<u32>,
    pub matrix_mult_4x3: WO<u32>,
    pub matrix_mult_3x3: WO<u32>,
    pub matrix_mult_scale: WO<u32>,
    pub matrix_mult_trans: WO<u32>,
    _0x74: [u32; 3],

    //VERTEX OPERATIONS 0x400_0480
    pub vertex_set_color: WO<u32>,
    pub vertex_set_normal: WO<u32>,
    pub vertex_set_texture_coordinate: WO<u32>,
    pub vertex_set_coordinate_double: WO<u32>,
    pub vertex_set_coordinate_single: WO<u32>,
    pub vertex_set_coordinate_xy: WO<u32>,
    pub vertex_set_coordinate_xz: WO<u32>,
    pub vertex_set_coordinate_yz: WO<u32>,
    pub vertex_set_coordinate_relative: WO<u32>,

    //MATERIAL OPERATIONS 0x400_04A4
    pub material_polygon_attributes: WO<PolygonAttributes>,
    pub material_texture_attributes: WO<u32>,
    pub material_color_palette: WO<u32>,
    _0xb0: [u8; 0x10],
    pub material_diffused_reflection: WO<u32>,
    pub material_specular_reflection: WO<u32>,

    //SHADER OPERATIONS 0x400_04C8
    pub shaders_light_vector: WO<u32>,
    pub shaders_light_color: WO<u32>,
    pub shaders_shiny_table: [WO<u8>; 0x20],
    _0xf0: [u8; 0x10],

    //PIPELINE OPERATIONS 0x400_0500
    pub pipeline_begin_vertex_list: WO<VertexListType>,
    pub pipeline_end_vertex_list: WO<u32>,
    _0x108: [u8; 0x38],
    pub pipeline_swap_buffers: WO<u32>,
    _0x144: [u8; 0x3C],
    pub pipeline_set_viewport: WO<Viewport>,
    _0x184: [u8; 0x3C],

    //TESTING OPERATIONS 0x400_05C0
    pub testing_box_test: WO<u32>,
    pub testing_pos_test: WO<u32>,
    pub testing_vec_test: WO<u32>,
}
impl GeometryCommands {
    #[inline]
    pub unsafe fn load_identity_matrix(&self) {
        self.matrix_identity.write(0);
    }
    #[inline]
    pub unsafe fn select_matrix_stack(&self, matrix_mode: MatrixMode) {
        self.matrix_mode.write(matrix_mode);
    }
    #[inline]
    pub unsafe fn start_vertex_list(&self, list_type: VertexListType) {
        self.pipeline_begin_vertex_list.write(list_type);
    }
    /// Ends the currently active vertex list
    ///
    /// Oddily, this is actually only here for debug purposes, and not actually required at all.
    #[inline]
    pub unsafe fn end_vertex_list(&self) {
        self.pipeline_end_vertex_list.write(0);
    }

    #[inline]
    pub unsafe fn load_matrix_4x4(&self, matrix: [[u32; 4]; 4]) {
        for row in matrix {
            for column in row {
                self.matrix_load_4x4.write(column);
            }
        }
    }
    //#[inline]
    pub unsafe fn scale_matrix(&self, x: i32, y: i32, z: i32) {
        self.matrix_mult_scale.write(x as u32);
        self.matrix_mult_scale.write(y as u32);
        self.matrix_mult_scale.write(z as u32);
    }
    //#[inline]
    pub unsafe fn translate_matrix(&self, x: i32, y: i32, z: i32) {
        self.matrix_mult_trans.write(x as u32);
        self.matrix_mult_trans.write(y as u32);
        self.matrix_mult_trans.write(z as u32);
    }
}
