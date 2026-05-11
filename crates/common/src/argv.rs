use crate::bootstrap::HeaderTWL;



pub const ARGV_MAGIC: i32 = 0x5f617267;
pub const SYSTEM_ARGV: *mut ArgvStructutre = 0x02FFFE70 as _;
//DKA argv struct
#[repr(C)]
pub struct ArgvStructutre {
    pub magic: i32,
    pub command_line: *mut u8,
    pub command_length: i32,
    pub argc: i32,
    pub argv: *mut *mut u8,
    pub dummy: i32,
    pub host: u32,
}

pub unsafe fn init(header: &HeaderTWL, file_path: &str) {
    //find argv location
    let ntr_arg_destination = (header.arm9_load + header.arm9_size + 7) & !3;
    let arg_destination = if header.is_dsi_mode() {
        let twl_arg_destination = (header.arm9i_load + header.arm9i_size + 7) & !3;
        ntr_arg_destination.max(twl_arg_destination)
    } else {
        ntr_arg_destination
    };

    //declare the final argv
    let argv = arg_destination as *mut u8;
    let mut argv_size: usize = 0;

    //insert rom path
    {
        for byte in file_path.as_bytes() {
            argv.add(argv_size).write_volatile(*byte);
            argv_size += 1;
        }
        argv.add(argv_size).write_volatile(0);
        argv_size += 1;
    }

    //"initialize" final structure
    let final_argv_structure = ArgvStructutre {
        magic: ARGV_MAGIC,
        command_line: argv,
        command_length: argv_size as i32,
        argc: 0,
        argv: core::ptr::null_mut(),
        dummy: 0,
        host: 0,
    };
    //Copy to it's final location
    SYSTEM_ARGV.write_volatile(final_argv_structure);
}