use std::marker::PhantomData;
use std::task::Poll;

use futures::{TryFuture, TryFutureExt, ready};
use heather::HBoxError;
use http::Request;
use pin_project_lite::pin_project;

use crate::handler::Handler;
use crate::{Error, IntoResponse, traits::*};

pub fn handle_fn<T, B, C, U>(func: T) -> HandleFn<T, B, C, U> {
    HandleFn(func, PhantomData)
}

pub struct HandleFn<T, B, C, U>(T, PhantomData<(B, C, U)>);

unsafe impl<T: Send, B, C, U> Send for HandleFn<T, B, C, U> {}

unsafe impl<T: Sync, B, C, U> Sync for HandleFn<T, B, C, U> {}

impl<T: Clone, B, C, U> Clone for HandleFn<T, B, C, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<T, B, C, U> Handler<B, C> for HandleFn<T, B, C, U>
where
    T: Fn(C, Request<B>) -> U + MaybeSendSync,
    U: TryFuture + MaybeSend,
    U::Ok: IntoResponse<B>,
    U::Error: Into<HBoxError<'static>>,
    B: MaybeSend,
    C: Clone,
{
    type Response = U::Ok;

    type Future<'a>
        = HandlerFnFuture<B, C, U>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: http::Request<B>) -> Self::Future<'a> {
        HandlerFnFuture {
            future: <U as TryFutureExt>::into_future((self.0)(context.clone(), req)),
            data: PhantomData,
        }
    }
}

pin_project! {
  pub struct HandlerFnFuture<B, C, U> {
    #[pin]
    future: futures::future::IntoFuture<U>,
    data: PhantomData<(B, C)>
  }
}

#[cfg(feature = "send")]
unsafe impl<B, C, U: Send> Send for HandlerFnFuture<B, C, U> {}

impl<B, C, U> Future for HandlerFnFuture<B, C, U>
where
    U: TryFuture,
    U::Error: Into<HBoxError<'static>>,
{
    type Output = Result<U::Ok, Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let ret = ready!(this.future.poll(cx));
        Poll::Ready(ret.map_err(Error::new))
    }
}
