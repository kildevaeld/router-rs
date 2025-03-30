#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod arena;
mod matcher;
mod params;
mod parser;
pub mod router;
mod segment;
mod segments;

pub use udled::Error as ParseError;

pub use self::{
    matcher::*,
    params::Params,
    parser::parse,
    router::{Route, Router},
    segment::Segment,
    segments::*,
};
