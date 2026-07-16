// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

///Code for interacting with the TI-TSC2117 Based Touch screen and dac controller on DSi

use core::marker::PhantomData;


pub struct ControlPage;
pub struct SoundPage;
pub struct CoeffPage;
pub struct TouchScreenPage;
pub struct SARBufferPage;
pub struct OperationPage;

pub trait TSCPage {
    const PAGE_NUMBER: u8;
}
pub trait TSCRegister: Into<u8> + From<u8> {
    const REGISTER_NUMBER: u8;
    type RegisterSelector;
}
pub trait TSCBuffer {
    const SIZE: usize;
    type RegisterSelector;
}
pub struct TSCHandle<T> {
    current_page: PhantomData<T>,
}
impl<T: TSCPage> TSCHandle<T> {
    pub unsafe fn write_register<R: TSCRegister<RegisterSelector = T>>(&self, value: R) {
        super::touchscreen::write_tsc(R::REGISTER_NUMBER, value.into());
    }
    pub unsafe fn read_register<R: TSCRegister<RegisterSelector = T>>(&self) -> R {
        R::from(super::touchscreen::read_tsc(R::REGISTER_NUMBER))
    }

    pub unsafe fn switch_page<U: TSCPage>(self, page: U) -> TSCHandle<U> {
        super::write_tsc(0, U::PAGE_NUMBER);
        TSCHandle {
            current_page: PhantomData,
        }
    }
}
