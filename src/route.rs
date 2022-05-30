use crate::{Params, Segment};

use super::parser::*;
#[cfg(not(feature = "std"))]
use alloc::{
    borrow::Cow,
    fmt,
    string::{String, ToString},
    vec::{IntoIter, Vec},
};
#[cfg(feature = "std")]
use std::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub struct Route<'a> {
    pub(crate) segments: Vec<Segment<'a>>,
}

impl<'a> Route<'a> {
    pub fn new(path: &'a str) -> Result<Route<'a>, ParseError> {
        Ok(Route {
            segments: parse(path)?,
        })
    }

    pub fn match_path<'b, P: Params<'b>>(&self, path: &'b str, params: &'b mut P) -> bool
    where
        'a: 'b,
    {
        match_path(&self.segments, path, params)
    }

    pub fn to_static(self) -> Route<'static> {
        Route {
            segments: self.segments.into_iter().map(|m| m.to_static()).collect(),
        }
    }
}

// impl<'a> AsSegments<'a> for Route<'a> {
//     type Error = std::convert::Infallible;
//     type Iter = Iter<'a, Segment<'a>>;
//     fn as_segments(&'a self) -> Result<Self::Iter, Self::Error> {
//         Ok(self.segments.iter())
//     }
// }
