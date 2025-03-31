use crate::parser::parse;
use crate::segment::Segment;
use alloc::{
    fmt,
    slice::Iter,
    string::String,
    vec::{IntoIter, Vec},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Segments<'a>(pub(crate) Vec<Segment<'a>>);

impl<'a> Segments<'a> {
    pub fn new(segments: Vec<Segment<'a>>) -> Segments<'a> {
        Segments(segments)
    }

    pub fn to_owned(self) -> Segments<'static> {
        Segments(self.0.into_iter().map(|m| m.to_owned()).collect())
    }

    pub fn iter<'b>(&'b self) -> Iter<'b, Segment<'a>> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> AsRef<[Segment<'a>]> for Segments<'a> {
    fn as_ref(&self) -> &[Segment<'a>] {
        self.0.as_ref()
    }
}

impl<'a> From<Segments<'a>> for Vec<Segment<'a>> {
    fn from(segs: Segments<'a>) -> Self {
        segs.0
    }
}

impl<'a> From<Vec<Segment<'a>>> for Segments<'a> {
    fn from(segs: Vec<Segment<'a>>) -> Self {
        Segments::new(segs)
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

        impl<'a> AsSegments<'a> for [Segment<'a>; $i] {
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

        impl<'a> AsSegments<'a> for [Segment<'a>; $i] {
            type Error = core::convert::Infallible;
            type Iter = IntoIter<Segment<'a>>;
            fn as_segments(self) -> Result<Self::Iter, Self::Error> {
                Ok(self.to_vec().into_iter())
            }
        }
    };
}

slice_impl!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22);

impl<'a> AsSegments<'a> for &'a str {
    type Error = udled::Error;
    type Iter = IntoIter<Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error> {
        let segments = parse(self)?;
        Ok(segments.into_iter())
    }
}

impl<'a> AsSegments<'a> for &'a String {
    type Error = udled::Error;
    type Iter = IntoIter<Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error> {
        let segments = parse(self)?;
        Ok(segments.into_iter())
    }
}

impl<'a> AsSegments<'a> for String {
    type Error = udled::Error;
    type Iter = IntoIter<Segment<'a>>;
    fn as_segments(self) -> Result<Self::Iter, Self::Error> {
        let segments = parse(&self)?;
        Ok(segments.to_owned().into_iter())
    }
}
