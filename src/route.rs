use super::parser::*;
#[cfg(not(feature = "std"))]
use alloc::{
    borrow::Cow,
    fmt,
    string::ToString,
    vec::{IntoIter, Vec},
};
#[cfg(feature = "std")]
use std::{
    borrow::Cow,
    fmt,
    string::ToString,
    vec::{IntoIter, Vec},
};
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

    pub fn constant(s: impl Into<Cow<'a, str>>) -> Segment<'a> {
        Segment::Constant(s.into())
    }

    pub fn parameter(s: impl Into<Cow<'a, str>>) -> Segment<'a> {
        Segment::Parameter(s.into())
    }

    pub fn star(s: impl Into<Cow<'a, str>>) -> Segment<'a> {
        Segment::Star(s.into())
    }
}

impl<'a> fmt::Display for Segment<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Segment::Constant(c) => f.write_str(c),
            Segment::Parameter(p) => write!(f, ":{}", p),
            Segment::Star(s) => write!(f, "*{}", s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segments<'a>(pub(crate) Vec<Segment<'a>>);

impl<'a> From<Segments<'a>> for Vec<Segment<'a>> {
    fn from(segs: Segments<'a>) -> Self {
        segs.0
    }
}

impl<'a> fmt::Display for Segments<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for v in self.0.iter() {
            write!(f, "/{}", v)?;
        }
        Ok(())
    }
}

impl<'a> IntoIterator for Segments<'a> {
    type Item = Segment<'a>;
    type IntoIter = IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> AsSegments<'a> for Segments<'a> {
    type Error = core::convert::Infallible;
    type Iter = IntoIter<Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error> {
        Ok(self.0.into_iter())
    }
}

pub trait AsSegments<'a> {
    type Error;
    type Iter: Iterator<Item = Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error>;
}

impl<'a> AsSegments<'a> for Vec<Segment<'a>> {
    type Error = core::convert::Infallible;
    type Iter = IntoIter<Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error> {
        Ok(self.into_iter())
    }
}

impl<'a, 'c> AsSegments<'a> for &'c [Segment<'a>] {
    type Error = core::convert::Infallible;
    type Iter = IntoIter<Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error> {
        Ok(self.to_vec().into_iter())
    }
}

macro_rules! slice_impl {
    ($i: literal) => {
        impl<'a, 'c> AsSegments<'a> for &'c [Segment<'a>; $i] {
            type Error = core::convert::Infallible;
            type Iter = IntoIter<Segment<'a>>;
            fn as_segments(self) -> Result<Self::Iter, Self::Error> {
                Ok(self.to_vec().into_iter())
            }
        }
    };
    ($i: literal, $($next: literal),*) => {
        slice_impl!($($next),*);
        impl<'a, 'c> AsSegments<'a> for &'c [Segment<'a>; $i] {
            type Error = core::convert::Infallible;
            type Iter = IntoIter<Segment<'a>>;
            fn as_segments(self) -> Result<Self::Iter, Self::Error> {
                Ok(self.to_vec().into_iter())
            }
        }
    };
}

slice_impl!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20);

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

impl<'a> AsSegments<'a> for &'a str {
    type Error = ParseError;
    type Iter = IntoIter<Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error> {
        let segments = parse(self)?;
        Ok(segments.into_iter())
    }
}
