use std::marker::PhantomData;

use crate::handler::{BoxHandler, Handler, box_handler};
use crate::traits::*;

pub trait Middleware<B, C, H>: MaybeSendSync {
    type Handle: Handler<B, C>;

    fn wrap(&self, handle: H) -> Self::Handle;
}

pub type BoxMiddleware<B, C, H> = Box<dyn Middleware<B, C, H, Handle = BoxHandler<B, C>>>;

pub fn box_middleware<B, C, M, H>(middleware: M) -> BoxMiddleware<B, C, H>
where
    M: Middleware<B, C, H> + 'static,
    M::Handle: 'static,
    B: MaybeSend + 'static,
    C: MaybeSendSync + 'static,
{
    Box::new(MiddlewareBox(middleware, PhantomData, PhantomData))
}

struct MiddlewareBox<B, C, T>(T, PhantomData<B>, PhantomData<C>);

impl<B, C, T: Clone> Clone for MiddlewareBox<B, C, T> {
    fn clone(&self) -> Self {
        MiddlewareBox(self.0.clone(), PhantomData, PhantomData)
    }
}

unsafe impl<B, C, T: Send> Send for MiddlewareBox<B, C, T> {}

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
