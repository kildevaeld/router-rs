use super::parser::*;
#[cfg(not(feature = "std"))]
use alloc::{borrow::Cow, string::ToString, vec::Vec};
#[cfg(feature = "std")]
use std::{borrow::Cow, string::ToString, vec::Vec};

#[derive(Debug, Clone, PartialEq)]
pub enum Segment<'a> {
    Constant(Cow<'a, str>),
    Parameter(Cow<'a, str>),
    Star(Cow<'a, str>),
}

impl<'a> Segment<'a> {
    pub fn to_static(self) -> Segment<'static> {
        match self {
            Segment::Constant(constant) => Segment::Constant(constant.to_string().into()),
            Segment::Parameter(param) => Segment::Parameter(param.to_string().into()),
            Segment::Star(star) => Segment::Star(star.to_string().into()),
        }
    }
}

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
