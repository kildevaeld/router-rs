#![no_std]

extern crate alloc;

mod container;
mod extensions;
pub mod modules;

pub use self::{container::*, extensions::Extensions};

pub mod prelude {
    pub use super::{Container, ReadableContainer};
}
