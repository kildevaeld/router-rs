use std::{marker::PhantomData, process::Output};

use futures_core::future;
use http::{Request, Response};

use crate::{
    Error,
    traits::{MaybeSend, MaybeSendSync},
};

#[cfg(feature = "send")]
pub type BoxFuture<'a, T> = future::BoxFuture<'a, T>;

#[cfg(not(feature = "send"))]
pub type BoxFuture<'a, T> = future::LocalBoxFuture<'a, T>;

pub trait Handler<B, C>: MaybeSendSync {
    type Future<'a>: Future<Output = Result<Response<B>, Error>> + MaybeSend
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a>;
}

pub trait DynHandler<B, C> {
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> BoxFuture<'a, Result<Response<B>, Error>>;
}

pub fn box_handler<C, B, T>(handler: T) -> BoxHandler<B, C>
where
    T: Handler<B, C> + 'static,
    B: 'static,
    C: 'static,
{
    Box::new(HandlerBox(handler, PhantomData, PhantomData))
}
pub type BoxHandler<B, C> = Box<dyn DynHandler<B, C>>;

pub struct HandlerBox<B, C, T>(T, PhantomData<C>, PhantomData<B>);

impl<B, C, T> DynHandler<B, C> for HandlerBox<B, C, T>
where
    T: Handler<B, C>,
{
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> BoxFuture<'a, Result<Response<B>, Error>> {
        Box::pin(async move { self.0.call(context, req).await })
    }
}
