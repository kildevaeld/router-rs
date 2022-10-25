#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod params;
mod parser;
mod route;
mod router;
mod segment;
mod segments;

pub use self::{
    params::*, parser::parse, parser::ParseError, route::*, router::*, segment::*, segments::*,
};
