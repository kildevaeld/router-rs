mod error;
// mod func;
mod handler;
mod handler_fn;
mod into_response;
mod middleware;
mod middleware_fn;
mod router;
#[cfg(feature = "tower")]
mod service_ext;
mod traits;

#[cfg(feature = "tower")]
pub use self::service_ext::ServiceExt;
pub use self::{
    error::Error,
    handler::{BoxHandler, Handler},
    handler_fn::{HandleFn, handle_fn},
    into_response::IntoResponse,
    middleware::Middleware,
    middleware_fn::{MiddlewareFn, middleware_fn},
    router::Builder,
};
