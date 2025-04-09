use core::fmt;

#[derive(Debug)]
pub struct Error {
    inner: Box<dyn core::error::Error + Send + Sync>,
}

impl Error {
    pub fn new<T: Into<Box<dyn core::error::Error + Send + Sync>>>(error: T) -> Error {
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

impl From<Box<dyn core::error::Error + Send + Sync>> for Error {
    fn from(value: Box<dyn core::error::Error + Send + Sync>) -> Self {
        Error { inner: value }
    }
}
