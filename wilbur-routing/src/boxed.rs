use std::marker::PhantomData;

use heather::{HBoxFuture, HSend, HSendSync, Hrc};
use hyper::{Request, Response};
use wilbur_core::{Handler, IntoResponse, Middleware};

use crate::error::Error;

pub trait DynHandler<B, C>: HSendSync {
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> HBoxFuture<'a, Result<Response<B>, Error>>;
}

pub fn box_handler<C, B, T>(handler: T) -> BoxHandler<B, C>
where
    T: Handler<B, C> + 'static,
    T::Error: Into<Error>,
    B: HSend + 'static,
    C: HSendSync + 'static,
{
    BoxHandler {
        inner: Hrc::from(HandlerBox(handler, PhantomData, PhantomData)),
    }
}

pub struct HandlerBox<B, C, T>(T, PhantomData<C>, PhantomData<B>);

unsafe impl<B, C, T: Send> Send for HandlerBox<B, C, T> {}

unsafe impl<B, C, T: Sync> Sync for HandlerBox<B, C, T> {}

impl<B, C, T> DynHandler<B, C> for HandlerBox<B, C, T>
where
    T: Handler<B, C> + 'static,
    T::Error: Into<Error>,
    C: HSendSync + 'static,
    B: HSend,
{
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> HBoxFuture<'a, Result<Response<B>, Error>> {
        Box::pin(async move {
            self.0
                .call(context, req)
                .await
                .map(|m| m.into_response())
                .map_err(Into::into)
        })
    }
}

pub struct BoxHandler<B, C> {
    inner: Hrc<dyn DynHandler<B, C>>,
}

unsafe impl<B, C> Send for BoxHandler<B, C> where Hrc<dyn DynHandler<B, C>>: Send {}

unsafe impl<B, C> Sync for BoxHandler<B, C> where Hrc<dyn DynHandler<B, C>>: Sync {}

impl<B, C> Handler<B, C> for BoxHandler<B, C> {
    type Response = Response<B>;
    type Error = Error;

    type Future<'a>
        = HBoxFuture<'a, Result<Self::Response, Self::Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a> {
        self.inner.call(context, req)
    }
}

impl<B, C> Clone for BoxHandler<B, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub fn box_middleware<B, C, M, H>(middleware: M) -> BoxMiddleware<B, C, H>
where
    M: Middleware<B, C, H> + 'static,
    M::Handler: 'static,
    <M::Handler as Handler<B, C>>::Error: Into<Error>,
    B: HSend + 'static,
    C: HSendSync + 'static,
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

unsafe impl<B, C, T: Send> Send for MiddlewareBox<B, C, T> {}

unsafe impl<B, C, T: Sync> Sync for MiddlewareBox<B, C, T> {}

impl<B, C, T, H> Middleware<B, C, H> for MiddlewareBox<B, C, T>
where
    T: Middleware<B, C, H>,
    T::Handler: 'static,
    <T::Handler as Handler<B, C>>::Error: Into<Error>,
    B: HSend + 'static,
    C: HSendSync + 'static,
{
    type Handler = BoxHandler<B, C>;

    fn wrap(&self, handle: H) -> Self::Handler {
        let handle = self.0.wrap(handle);
        box_handler(handle)
    }
}

pub struct BoxMiddleware<B, C, H> {
    inner: Hrc<dyn Middleware<B, C, H, Handler = BoxHandler<B, C>>>,
}

unsafe impl<B, C, H> Send for BoxMiddleware<B, C, H> where
    Hrc<dyn Middleware<B, C, H, Handler = BoxHandler<B, C>>>: Send
{
}

unsafe impl<B, C, H> Sync for BoxMiddleware<B, C, H> where
    Hrc<dyn Middleware<B, C, H, Handler = BoxHandler<B, C>>>: Sync
{
}

impl<B, C, H> Middleware<B, C, H> for BoxMiddleware<B, C, H> {
    type Handler = BoxHandler<B, C>;

    fn wrap(&self, handle: H) -> Self::Handler {
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
