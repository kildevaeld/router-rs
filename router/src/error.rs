use core::fmt;

use heather::HBoxError;

#[derive(Debug)]
pub struct Error {
    inner: HBoxError<'static>,
}

impl Error {
    pub fn new<T: Into<HBoxError<'static>>>(error: T) -> Error {
        Error {
            inner: error.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

impl From<routing::ParseError> for Error {
    fn from(value: routing::ParseError) -> Self {
        Error::new(value)
    }
}

impl From<routing::router::RouteError> for Error {
    fn from(value: routing::router::RouteError) -> Self {
        Error::new(value)
    }
}

impl From<HBoxError<'static>> for Error {
    fn from(value: HBoxError<'static>) -> Self {
        Error { inner: value }
    }
}
