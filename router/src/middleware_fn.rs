use crate::handler::Handler;
use crate::{Middleware, traits::*};
use std::marker::PhantomData;

pub fn middleware_fn<T, B, C, H, U>(func: T) -> MiddlewareFn<T, B, C, H, U> {
    MiddlewareFn(func, PhantomData)
}

pub struct MiddlewareFn<T, B, C, H, U>(T, PhantomData<(B, C, H, U)>);

unsafe impl<T: Send, B, C, H, U> Send for MiddlewareFn<T, B, C, H, U> {}

unsafe impl<T: Sync, B, C, H, U> Sync for MiddlewareFn<T, B, C, H, U> {}

impl<T: Clone, B, C, H, U> Clone for MiddlewareFn<T, B, C, H, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T, B, C, H, U> Middleware<B, C, H> for MiddlewareFn<T, B, C, H, U>
where
    T: Fn(H) -> U + MaybeSendSync,
    U: Handler<B, C>,
    B: MaybeSend,
{
    type Handle = U;

    fn wrap(&self, handle: H) -> Self::Handle {
        (self.0)(handle)
    }
}
