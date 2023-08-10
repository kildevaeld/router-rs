use std::{
    borrow::Cow,
    fmt,
    string::{ToString},
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

    pub fn as_str(&self) -> &str {
        match self {
            Segment::Constant(c) => c,
            Segment::Parameter(p) => p,
            Segment::Star(s) => s,
        }
    }

    pub fn into_inner(self) -> Cow<'a, str> {
        match self {
            Segment::Constant(c) => c,
            Segment::Parameter(p) => p,
            Segment::Star(s) => s,
        }
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
