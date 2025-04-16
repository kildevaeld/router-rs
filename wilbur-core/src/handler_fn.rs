use core::marker::PhantomData;

use heather::{HBoxFuture, HSend, HSendSync};
use http::{Request, Response};

use crate::{Error, FromRequest, Handler, IntoResponse};

pub fn handler<B, C, T, I>(func: T) -> FuncHandler<T, I, B, C>
where
    T: Func<B, C, I>,
    <T::Future as Future>::Output: IntoResponse<B> + HSend,
    I: FromRequest<B, C>,
    B: 'static,
{
    FuncHandler {
        func,
        i: PhantomData,
    }
}

pub struct FuncHandler<T, I, B, C> {
    func: T,
    i: PhantomData<(B, C, I)>,
}

impl<T, I, B, C> Clone for FuncHandler<T, I, B, C>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        FuncHandler {
            func: self.func.clone(),
            i: PhantomData,
        }
    }
}

unsafe impl<T, I, B, C> Send for FuncHandler<T, I, B, C> where T: Send {}

unsafe impl<T, I, B, C> Sync for FuncHandler<T, I, B, C> where T: Sync {}

impl<B, C, T, I> Handler<B, C> for FuncHandler<T, I, B, C>
where
    T: Func<B, C, I> + HSendSync,
    T::Future: HSend,
    <T::Future as Future>::Output: IntoResponse<B> + HSend,
    I: FromRequest<B, C>,
    for<'a> I::Future<'a>: HSend,
    B: 'static + HSend,
    C: HSendSync,
{
    type Response = Response<B>;

    type Future<'a>
        = HBoxFuture<'a, Result<Self::Response, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a> {
        Box::pin(async move {
            let ret = I::from_request(req, context).await?;
            let resp = self.func.call(ret).await;
            Ok(resp.into_response())
        })
    }
}

pub trait Func<B, C, I> {
    type Future: Future;

    fn call(&self, req: I) -> Self::Future;
}

impl<B, C, F, U> Func<B, C, ()> for F
where
    F: Fn() -> U,
    U: Future,
{
    type Future = U;

    fn call(&self, _req: ()) -> Self::Future {
        (self)()
    }
}

macro_rules! funcs {
    ($first: ident) => {
        impl<B, C, F, U, $first> Func<B, C, ($first,)> for F
        where
            F: Fn($first) -> U,
            U: Future,
            $first: FromRequest<B, C>
        {
            type Future = U;

            fn call(&self, req: ($first,)) -> Self::Future {
                (self)(req.0)
            }
        }
    };
    ($first: ident, $($rest:ident),+) => {
        funcs!($($rest),+);

        impl<B, C, F, U, $first, $($rest),*> Func<B, C, ($first, $($rest),*)> for F
        where
            F: Fn($first, $($rest),*) -> U,
            U: Future,

        {
            type Future = U;

            #[allow(non_snake_case)]
            fn call(&self, ($first, $($rest),*): ($first,$($rest),*)) -> Self::Future {
                (self)($first, $($rest),*)
            }
        }
    }
}

funcs!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
