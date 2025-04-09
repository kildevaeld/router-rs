mod error;
// mod func;
mod handler;
mod handler_fn;
mod into_response;
mod middleware;
mod middleware_fn;
mod modifier;
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
    middleware::{BoxMiddleware, Middleware, PathMiddleware},
    middleware_fn::{MiddlewareFn, middleware_fn},
    modifier::{BoxModifier, BoxModify, Modifier, ModifierList, Modify},
    router::{Builder, Router, UrlParams, compose},
    traits::Routing,
};

pub use routing::{Params, router::MethodFilter};
use uhuh_container::modules::BuildContext;

pub trait RouterBuildContext: BuildContext {
    type Body;
}
