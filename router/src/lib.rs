use handler::{BoxHandler, Handler};
use http::{Method, Request, Response};
use tower::Service;

mod handler;
struct Error {}

#[cfg(not(feature = "send"))]
type BoxService<B> = tower_util::UnsyncBoxService<Request<B>, Response<B>, Error>;

#[cfg(feature = "send")]
type BoxService<B> = tower_util::BoxService<Request<B>, Response<B>, Error>;

bitflags::bitflags! {
    pub struct MethodFilter: u8 {
       const GET = 1 << 0;
       const POST = 1 << 1;
       const PUT = 1 << 2;
       const PATCH = 1 << 3;
       const DELETE = 1 << 4;
    }
}

impl MethodFilter {
    pub fn any() -> MethodFilter {
        MethodFilter::all()
    }
}

pub struct RouteHandler<C, B> {
    method: MethodFilter,
    handler: BoxHandler<B, C>,
    name: Option<String>,
}

pub struct Router<C, B> {
    tree: routing::Router<RouteHandler<C, B>>,
}

impl<C: 'static, B: 'static> Router<C, B> {
    pub fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T)
    where
        T: Handler<B, C> + 'static,
    {
    }
}
