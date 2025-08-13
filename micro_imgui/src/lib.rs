#![no_std]
#![feature(nonzero_ops)]

use context::{Ctx, Frame};
extern crate alloc;

mod context;
mod event;
mod primitives;
mod response;
mod ui;
pub mod widgets;
pub use primitives::*;
pub use response::*;

pub fn run<B: Backend, T>(backend: B, mut start: T, mut fun: impl FnMut(&mut Frame<B>, &mut T)) {
    let mut context = Ctx::new(backend);
    loop {
        context.process_frame(&mut fun, &mut start);
        while !(context.backend.gather_inputs() || context.wants_repaint) {}
    }
}
