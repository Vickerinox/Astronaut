#![no_std]

macro_rules! const_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}

pub mod argv;
pub mod blowfish;
pub mod bootstrap;
pub mod bootstub;
pub mod config;
pub mod device_list;
pub mod modcrypt;
pub mod patching;
