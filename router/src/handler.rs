use http::{Request, Response};
use std::marker::PhantomData;

use crate::{
    error::Error,
    traits::{BoxFuture, MaybeSend, MaybeSendSync},
};

pub trait Handler<B, C>: MaybeSendSync {
    type Future<'a>: Future<Output = Result<Response<B>, Error>> + MaybeSend
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
    Box::new(HandlerBox(handler, PhantomData, PhantomData))
}
pub type BoxHandler<B, C> = Box<dyn DynHandler<B, C>>;

pub struct HandlerBox<B, C, T>(T, PhantomData<C>, PhantomData<B>);

unsafe impl<B, C, T: Send> Send for HandlerBox<B, C, T> {}

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
        Box::pin(async move { self.0.call(context, req).await })
    }
}

impl<B, C> Handler<B, C> for BoxHandler<B, C> {
    type Future<'a>
        = BoxFuture<'a, Result<Response<B>, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a> {
        (**self).call(context, req)
    }
}
