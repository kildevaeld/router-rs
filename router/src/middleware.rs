use std::marker::PhantomData;

use heather::Hrc;

use crate::handler::{BoxHandler, Handler, box_handler};
use crate::traits::*;

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
