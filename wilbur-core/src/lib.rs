pub mod body;
mod error;
mod from_request;
pub mod handler;
mod into_response;
pub mod middleware;
mod middleware_fn;
pub mod modifier;

mod handler_fn;

pub use self::{
    error::Error,
    from_request::{FromRequest, FromRequestParts},
    handler::Handler,
    handler_fn::{FuncHandler, handler},
    into_response::IntoResponse,
    middleware::Middleware,
    middleware_fn::{MiddlewareFn, middleware},
    modifier::{Modifier, Modify},
};

// Export stuff that maybe should be handled by this crate
pub use http::{Request, Response};

// Export common http types
pub use bytes::Bytes;
pub use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
