#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod arena;
mod matcher;
mod params;
mod parser;
pub mod path_router;
mod segment;
mod segments;

pub use udled::Error as ParseError;

pub use self::{
    matcher::*,
    params::Params,
    parser::parse,
    path_router::{PathRouter, Route},
    segment::Segment,
    segments::*,
};

#[cfg(feature = "router")]
pub mod router;
