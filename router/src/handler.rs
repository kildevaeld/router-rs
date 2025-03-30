use std::process::Output;

use futures_core::future::BoxFuture;
use http::{Request, Response};

use crate::Error;

pub trait Handler<B, C> {
    type Future<'a>: Future<Output = Result<Response<B>, Error>>
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

pub type BoxHandler<B, C> = Box<dyn DynHandler<B, C>>;
