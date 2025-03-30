use heather::Hrc;
use http::{Request, Response};
use std::marker::PhantomData;

use crate::{
    IntoResponse,
    error::Error,
    traits::{BoxFuture, MaybeSend, MaybeSendSync},
};

pub trait Handler<B, C>: MaybeSendSync {
    type Response: IntoResponse<B>;
    type Future<'a>: Future<Output = Result<Self::Response, Error>> + MaybeSend
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a>;
}

pub trait DynHandler<B, C>: MaybeSendSync {
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> BoxFuture<'a, Result<Response<B>, Error>>;
}

pub fn box_handler<C, B, T>(handler: T) -> BoxHandler<B, C>
where
    T: Handler<B, C> + 'static,
    B: MaybeSend + 'static,
    C: MaybeSendSync + 'static,
{
    BoxHandler {
        inner: Hrc::from(HandlerBox(handler, PhantomData, PhantomData)),
    }
}

pub struct HandlerBox<B, C, T>(T, PhantomData<C>, PhantomData<B>);

#[cfg(feature = "send")]
unsafe impl<B, C, T: Send> Send for HandlerBox<B, C, T> {}

#[cfg(feature = "send")]
unsafe impl<B, C, T: Sync> Sync for HandlerBox<B, C, T> {}

impl<B, C, T> DynHandler<B, C> for HandlerBox<B, C, T>
where
    T: Handler<B, C> + 'static,
    C: MaybeSendSync + 'static,
    B: MaybeSend,
{
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> BoxFuture<'a, Result<Response<B>, Error>> {
        Box::pin(async move { self.0.call(context, req).await.map(|m| m.into_response()) })
    }
}

pub struct BoxHandler<B, C> {
    inner: Hrc<dyn DynHandler<B, C>>,
}

#[cfg(feature = "send")]
unsafe impl<B, C> Send for BoxHandler<B, C> {}

#[cfg(feature = "send")]
unsafe impl<B, C> Sync for BoxHandler<B, C> {}

impl<B, C> Handler<B, C> for BoxHandler<B, C> {
    type Response = Response<B>;

    type Future<'a>
        = BoxFuture<'a, Result<Self::Response, Error>>
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
