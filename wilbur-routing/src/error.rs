#[derive(Debug)]
pub enum RouteError {
    Parse(routing::ParseError),
    Route(routing::router::RouteError),
}

impl From<routing::ParseError> for RouteError {
    fn from(value: routing::ParseError) -> Self {
        RouteError::Parse(value)
    }
}

impl From<routing::router::RouteError> for RouteError {
    fn from(value: routing::router::RouteError) -> Self {
        RouteError::Route(value)
    }
}
