use std::marker::PhantomData;

use heather::Hrc;
use http::Response;
use routing::{AsSegments, Segments};

use crate::handler::{BoxHandler, Handler, box_handler};
use crate::{IntoResponse, traits::*};

pub trait Middleware<B, C, H>: MaybeSendSync {
    type Handle: Handler<B, C>;

    fn wrap(&self, handle: H) -> Self::Handle;
}

pub fn box_middleware<B, C, M, H>(middleware: M) -> BoxMiddleware<B, C, H>
where
    M: Middleware<B, C, H> + 'static,
    M::Handle: 'static,
    B: MaybeSend + 'static,
    C: MaybeSendSync + 'static,
{
    BoxMiddleware {
        inner: Hrc::from(MiddlewareBox(middleware, PhantomData)),
    }
}

struct MiddlewareBox<B, C, T>(T, PhantomData<(B, C)>);

impl<B, C, T: Clone> Clone for MiddlewareBox<B, C, T> {
    fn clone(&self) -> Self {
        MiddlewareBox(self.0.clone(), PhantomData)
    }
}

#[cfg(feature = "send")]
unsafe impl<B, C, T: Send> Send for MiddlewareBox<B, C, T> {}

#[cfg(feature = "send")]
unsafe impl<B, C, T: Sync> Sync for MiddlewareBox<B, C, T> {}

impl<B, C, T, H> Middleware<B, C, H> for MiddlewareBox<B, C, T>
where
    T: Middleware<B, C, H>,
    T::Handle: 'static,
    B: MaybeSend + 'static,
    C: MaybeSendSync + 'static,
{
    type Handle = BoxHandler<B, C>;

    fn wrap(&self, handle: H) -> Self::Handle {
        let handle = self.0.wrap(handle);
        box_handler(handle)
    }
}

pub struct BoxMiddleware<B, C, H> {
    inner: Hrc<dyn Middleware<B, C, H, Handle = BoxHandler<B, C>>>,
}

#[cfg(feature = "send")]
unsafe impl<B, C, H> Send for BoxMiddleware<B, C, H> {}

#[cfg(feature = "send")]
unsafe impl<B, C, H> Sync for BoxMiddleware<B, C, H> {}

impl<B, C, H> Middleware<B, C, H> for BoxMiddleware<B, C, H> {
    type Handle = BoxHandler<B, C>;

    fn wrap(&self, handle: H) -> Self::Handle {
        self.inner.wrap(handle)
    }
}

impl<B, C, H> Clone for BoxMiddleware<B, C, H> {
    fn clone(&self) -> Self {
        BoxMiddleware {
            inner: self.inner.clone(),
        }
    }
}

pub struct Passthrough;

impl<B, C, H> Middleware<B, C, H> for Passthrough
where
    H: Handler<B, C>,
{
    type Handle = H;
    fn wrap(&self, handle: H) -> Self::Handle {
        handle
    }
}

#[derive(Debug, Clone)]
pub struct PathMiddleware<M> {
    path: Segments<'static>,
    middleware: M,
}

impl<M> PathMiddleware<M> {
    pub fn new<'a, S: AsSegments<'a>>(
        path: S,
        middleware: M,
    ) -> Result<PathMiddleware<M>, S::Error> {
        let segments = path.as_segments()?;
        Ok(PathMiddleware {
            path: segments.map(|m| m.to_owned()).collect::<Vec<_>>().into(),
            middleware,
        })
    }
}

impl<B, C, M, H> Middleware<B, C, H> for PathMiddleware<M>
where
    H: Handler<B, C> + Clone,
    M: Middleware<B, C, H>,
    B: 'static,
{
    type Handle = PathMiddlewareService<H, M::Handle>;

    fn wrap(&self, handler: H) -> Self::Handle {
        PathMiddlewareService {
            wrapped_handler: self.middleware.wrap(handler.clone()),
            handler,
            segments: self.path.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathMiddlewareService<T, M> {
    handler: T,
    wrapped_handler: M,
    segments: Segments<'static>,
}

impl<B: 'static, C, T, M> Handler<B, C> for PathMiddlewareService<T, M>
where
    T: Handler<B, C>,
    T::Response: IntoResponse<B>,
    M: Handler<B, C>,
    M::Response: IntoResponse<B>,
{
    type Response = Response<B>;

    type Future<'a>
        = heather::HBoxFuture<'a, Result<Self::Response, crate::Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: http::Request<B>) -> Self::Future<'a> {
        Box::pin(async move {
            if routing::match_path(&self.segments, req.uri().path(), &mut ()) {
                Ok(self
                    .wrapped_handler
                    .call(context, req)
                    .await?
                    .into_response())
            } else {
                Ok(self.handler.call(context, req).await?.into_response())
            }
        })
    }
}
