#![no_std]

macro_rules! const_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}

pub mod bootstrap;
pub mod config;
pub mod argv;
pub mod device_list;
pub mod modcrypt;
pub mod blowfish;

