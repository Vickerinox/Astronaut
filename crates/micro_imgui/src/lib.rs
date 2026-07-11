#![no_std]
#![feature(nonzero_ops)]

pub use context::{Ctx, Frame};
pub use ui::Ui;
extern crate alloc;

mod context;
mod event;
mod primitives;
mod response;
mod ui;
pub mod widgets;
pub use context::{ColorSet, Style};
pub use primitives::*;
pub use response::*;

pub trait Application<B: Backend> {}
pub fn run<B: Backend, T>(
    backend: B,
    style: Style,
    mut start: T,
    mut fun: impl FnMut(Frame<B>, &mut T),
    mut background: impl FnMut(&mut T),
) {
    let mut context = Ctx::new(backend, style);
    loop {
        context.process_frame(&mut fun, &mut start);
        loop {
            background(&mut start);
            if context.backend.gather_inputs() || context.wants_repaint {
                break;
            }
        }
        context.wants_repaint = false;
    }
}
