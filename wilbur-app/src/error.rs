use core::fmt;

#[derive(Debug)]
pub struct Error {
    inner: Box<dyn core::error::Error + Send + Sync>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

impl From<Box<dyn core::error::Error + Send + Sync>> for Error {
    fn from(value: Box<dyn core::error::Error + Send + Sync>) -> Self {
        Error { inner: value }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error {
            inner: value.into(),
        }
    }
}

#[cfg(feature = "hyper")]
impl From<hyper::Error> for Error {
    fn from(value: hyper::Error) -> Self {
        Error {
            inner: value.into(),
        }
    }
}
