#[derive(Debug)]
pub enum Error {
    Parse(routing::ParseError),
    Route(routing::router::RouteError),
}

impl From<routing::ParseError> for Error {
    fn from(value: routing::ParseError) -> Self {
        Error::Parse(value)
    }
}

impl From<routing::router::RouteError> for Error {
    fn from(value: routing::router::RouteError) -> Self {
        Error::Route(value)
    }
}
