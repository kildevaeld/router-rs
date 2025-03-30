use std::future::poll_fn;

use http::{Request, Response};
use tower::Service;
use tower_util::{Oneshot, ServiceExt as _};

use crate::{
    error::Error,
    handler::Handler,
    into_response::IntoResponse,
    traits::{BoxFuture, MaybeSend, MaybeSendSync},
};

pub trait ServiceExt<R>: Service<R> {
    fn into_handle(self) -> ServiceHandle<Self>
    where
        Self: Sized,
    {
        ServiceHandle(self)
    }
}

impl<R, T> ServiceExt<R> for T where T: Service<R> {}

pub struct ServiceHandle<T>(T);

impl<T, C, B> Handler<B, C> for ServiceHandle<T>
where
    T: Service<Request<B>> + MaybeSendSync + Clone,
    T::Future: MaybeSend,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    T::Response: IntoResponse<B>,
    B: MaybeSend + 'static,
{
    type Future<'a>
        = BoxFuture<'a, Result<Response<B>, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, _context: &'a C, req: Request<B>) -> Self::Future<'a> {
        Box::pin(async move {
            Ok(self
                .0
                .clone()
                .oneshot(req)
                .await
                .map_err(Error::new)?
                .into_response())
        })
    }
}
