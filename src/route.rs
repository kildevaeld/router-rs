use super::parser::*;
use crate::parser::match_path;
use crate::{Params, Segments};
#[cfg(not(feature = "std"))]
use alloc::{
    borrow::Cow,
    fmt,
    string::{String, ToString},
    vec::{IntoIter, Vec},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Route<'a> {
    pub(crate) segments: Segments<'a>,
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
        match_path(&self.segments.0, path, params)
    }

    pub fn to_static(self) -> Route<'static> {
        Route {
            segments: self.segments.to_static(),
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
