#![no_std]

#[cfg(not(feature = "std"))]
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
    params::*,
    parser::{parse, ParseError},
    route::*,
    router::*,
    segment::*,
    segments::*,
};

// #[cfg(feature = "http")]
// mod http_ext;

// #[cfg(feature = "http")]
// pub use http_ext::*;
