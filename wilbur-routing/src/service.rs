use heather::{HBoxFuture, HSend, HSendSync, Hrc};
use hyper::{Request, Response};

use crate::{error::Error, router::Router};

pub struct RouterService<B, C> {
    pub(crate) router: Hrc<Router<B, C>>,
    pub(crate) context: C,
}

#[cfg(feature = "tower")]
impl<B, C> tower::Service<Request<B>> for RouterService<B, C>
where
    B: HSend + 'static,
    C: Clone + HSendSync + 'static,
{
    type Response = Response<B>;

    type Error = Error;

    type Future = HBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        core::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { this.router.handle(req, &this.context).await })
    }
}

#[cfg(feature = "hyper")]
impl<B, C> hyper::service::Service<Request<B>> for RouterService<B, C>
where
    B: HSend + 'static,
    C: Clone + HSendSync + 'static,
{
    type Response = Response<B>;

    type Error = Error;

    type Future = HBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&self, req: Request<B>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { this.router.handle(req, &this.context).await })
    }
}

impl<B, C> Clone for RouterService<B, C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            context: self.context.clone(),
        }
    }
}
