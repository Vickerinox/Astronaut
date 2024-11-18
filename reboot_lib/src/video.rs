use volatile_register::{RO, RW, WO};
use bitflags::bitflags;
use crate::RegisterWrapper;

///Public access to the DS video hardware registers
pub const VIDEO_HARDWARE: RegisterWrapper<VideoHardware> = RegisterWrapper(0x0400_0000 as *mut VideoHardware);


bitflags! {
    #[derive(Clone, Copy)]
    pub struct PrimaryDisplayControl: u32 {
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
        Self::from_bits_retain((x0 as u32) | ((y0 as u32) << 8) | ((x1 as u32) << 16) | ((y1 as u32) << 24))
    }
}

#[repr(C)]
pub struct VideoHardware {
    pub primary_display_control: RW::<PrimaryDisplayControl>,
    _unimplemented: [u8; 0x5C],
    pub display_control_3d: RW<u16>,
    _unimplemented2: [u8; 0x1DE],
    pub vram_control_bank_a: WO<u8>,
    pub vram_control_bank_b: WO<u8>,
    pub vram_control_bank_c: WO<u8>,
    pub vram_control_bank_d: WO<u8>,
    pub vram_control_bank_e: WO<u8>,
    pub vram_control_bank_f: WO<u8>,
    pub vram_control_bank_g: WO<u8>,
    _vram_empty: u8,
    pub vram_control_bank_h: WO<u8>,
    pub vram_control_bank_i: WO<u8>,
    _unimplemented_3: [u8; 0xD6],
    pub rendered_lines: RO<u8>,
    _free_bus: [u8; 15],
    pub edge_colors: [WO<u16>; 8],
    pub alpha_test_ref: WO<u8>,
    _free_bus2: [u8; 15],
    pub clear_color: WO<u32>,
    pub clear_depth: WO<u16>,
    pub clear_offset: WO<u16>,
    pub fog_color: WO<u32>,
    pub fog_offset: WO<u16>,
    _free_bus3: u16,
    pub fog_table: [WO<u8>; 0x20],
    pub toon_table: [WO<u16>; 0x20],
    _free_bus_4: [u8; 0x40],
    pub geometry_commands: GeometryCommands,
    _free_bus_5:[u8; 0x34],
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

    _free_bus: [u32; 3],

    //VERTEX OPERATIONS 0x400_0480
    pub vertex_set_color: WO<u32>,
    pub vertex_set_normal: WO<u32>,
    pub vertex_set_texture_coordinate: WO<u32>,
    pub vertex_set_coordinate_single: WO<u32>,
    pub vertex_set_coordinate_double: WO<u32>,
    pub vertex_set_coordinate_xy: WO<u32>,
    pub vertex_set_coordinate_xz: WO<u32>,
    pub vertex_set_coordinate_yz: WO<u32>,
    pub vertex_set_coordinate_relative: WO<u32>,

    //MATERIAL OPERATIONS 0x400_04A4
    pub material_polygon_attributes: WO<u32>,
    pub material_texture_attributes: WO<u32>,
    pub material_color_palette: WO<u32>,
    _free_bus_2: [u8; 0x10],
    pub material_diffused_reflection: WO<u32>,
    pub material_specular_reflection: WO<u32>,

    //SHADER OPERATIONS 0x400_04C8
    pub shaders_light_vector: WO<u32>,
    pub shaders_light_color: WO<u32>,
    pub shaders_shiny_table: [WO<u8>; 0x20],
    _free_bus_3: [u8; 0x10],

    //PIPELINE OPERATIONS 0x400_0500
    pub pipeline_begin_vertex_list: WO<VertexListType>,
    pub pipeline_end_vertex_list: WO<u32>,
    _free_bus4: [u8; 0x38],
    pub pipeline_swap_buffers: WO<u32>,
    _free_bus5: [u8; 0x3C],
    pub pipeline_set_viewport: WO<Viewport>,
    _free_bus6: [u8; 0x3C],
    

    //TESTING OPERATIONS 0x400_05C0
    pub testing_box_test: WO<u32>,
    pub testing_pos_test: WO<u32>,
    pub testing_vec_test: WO<u32>,
}