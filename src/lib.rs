#![no_std]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod parser;
mod route;
mod router;

pub use self::{
    parser::{Params, ParseError},
    route::*,
    router::*,
};
