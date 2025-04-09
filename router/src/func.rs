use std::marker::PhantomData;

use http::{Request, Response, request::Parts};

use crate::{Error, Handler, IntoResponse};
use heather::{HBoxFuture, HSend};

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

pub trait FromRequestParts<C>: Sized {
    type Future<'a>: Future<Output = Result<Self, Error>>
    where
        C: 'a;

    fn from_request_parts<'a>(parts: &'a mut Parts, state: &'a C) -> Self::Future<'a>;
}

pub trait FromRequest<B, C>: Sized {
    type Future<'a>: Future<Output = Result<Self, Error>>
    where
        C: 'a,
        B: 'a;

    fn from_request<'a>(parts: Request<B>, state: &'a C) -> Self::Future<'a>;
}

impl<C: 'static, B: 'static> FromRequest<B, C> for Request<B> {
    type Future<'a> = futures::future::Ready<Result<Self, Error>>;

    fn from_request<'a>(parts: Request<B>, _state: &'a C) -> Self::Future<'a> {
        futures::future::ready(Ok(parts))
    }
}

impl<C: 'static, B: 'static> FromRequest<B, C> for () {
    type Future<'a> = futures::future::Ready<Result<(), Error>>;

    fn from_request<'a>(_parts: Request<B>, _state: &'a C) -> Self::Future<'a> {
        futures::future::ready(Ok(()))
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

funcs!(T1, T2, T3, T4, T5, T6, T7, T8);

pub struct FuncHandler<T, I, B, C> {
    func: T,
    i: PhantomData<(B, C, I)>,
}

impl<B, C, T, I> Handler<B, C> for FuncHandler<T, I, B, C>
where
    T: Func<B, C, I>,
    <T::Future as Future>::Output: IntoResponse<B> + HSend,
    I: FromRequest<B, C>,
    B: 'static,
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

macro_rules! from_request {
    ($first: ident) => {
        impl<B, C, $first> FromRequest<B, C> for ($first,)
        where
            C: 'static,
            B: 'static,
            $first: FromRequest<B, C>,
        {
            type Future<'a> = HBoxFuture<'a, Result<($first,), Error>>;

            fn from_request<'a>(parts: Request<B>, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {
                    let ret = $first::from_request(parts, state).await?;
                    Ok((ret,))
                })
            }
        }
    };
    ($first: ident, $($rest:ident),+) => {
        from_request!($($rest),+);
        impl<B, C, $first, $($rest),*> FromRequest<B, C> for ($($rest),+,$first)
        where
            C: 'static,
            B: 'static,
            $first: FromRequest<B, C>,
            $(
                $rest: FromRequestParts<C>
            ),+
        {
            type Future<'a> = HBoxFuture<'a, Result<($($rest),+,$first), Error>>;

            #[allow(non_snake_case, unused_parens)]
            fn from_request<'a>(req: Request<B>, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {

                    let (mut parts, data) = req.into_parts();

                    let ($($rest),+) = (
                        $(
                            $rest::from_request_parts(&mut parts, state).await?
                        ),+
                    );
                    let ret = $first::from_request(Request::from_parts(parts, data), state).await?;
                    Ok(($($rest),+,ret))
                })
            }
        }
    };
}

from_request!(T1, T2, T3, T4, T5, T6, T7, T8);

pub fn handler<B, C, T, I>(func: T) -> FuncHandler<T, I, B, C>
where
    T: Func<B, C, I>,
    <T::Future as Future>::Output: IntoResponse<B> + HSend,
    I: FromRequest<B, C>,
    B: 'static,
{
    FuncHandler {
        func: func,
        i: PhantomData,
    }
}
