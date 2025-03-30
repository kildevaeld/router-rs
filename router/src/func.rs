use http::{Request, request::Parts};

use crate::Error;

pub trait Func<C, I> {
    type Future: Future;

    fn call(&self, req: I) -> Self::Future;
}

impl<C, F, U> Func<C, ()> for F
where
    F: Fn() -> U,
    U: Future,
{
    type Future = U;

    fn call(&self, req: ()) -> Self::Future {
        (self)()
    }
}

pub trait FromRequestParts<C> {
    type Future<'a>: Future
    where
        C: 'a;

    fn from_request_parts<'a>(parts: &'a mut Parts, state: &'a C) -> Self::Future<'a>;
}

pub trait FromRequest<C, B>: Sized {
    type Future<'a>: Future<Output = Result<Self, Error>>
    where
        C: 'a,
        B: 'a;

    fn from_request<'a>(parts: Request<B>, state: &'a C) -> Self::Future<'a>;
}

impl<C: 'static, B: 'static> FromRequest<C, B> for Request<B> {
    type Future<'a> = futures::future::Ready<Result<Self, Error>>;

    fn from_request<'a>(parts: Request<B>, state: &'a C) -> Self::Future<'a> {
        futures::future::ready(Ok(parts))
    }
}

macro_rules! funcs {
    ($first: ident) => {
        impl<C, F, U, $first> Func<C, ($first,)> for F
        where
            F: Fn($first) -> U,
            U: Future,
        {
            type Future<'a> = BoxFuture<'a, U::Output>;

            fn call(&self, req: ($first,)) -> Self::Future {}
        }
    };
}
