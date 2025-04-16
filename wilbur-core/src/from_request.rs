use heather::{HBoxFuture, HSend, HSendSync};
use http::{HeaderMap, Request, Uri, request::Parts};

use crate::Error;

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
    type Future<'a> = core::future::Ready<Result<Self, Error>>;

    fn from_request<'a>(parts: Request<B>, _state: &'a C) -> Self::Future<'a> {
        core::future::ready(Ok(parts))
    }
}

impl<C: 'static, B: 'static> FromRequest<B, C> for () {
    type Future<'a> = core::future::Ready<Result<(), Error>>;

    fn from_request<'a>(_parts: Request<B>, _state: &'a C) -> Self::Future<'a> {
        core::future::ready(Ok(()))
    }
}

macro_rules! from_request {
    ($first: ident) => {
        impl<B, C, $first> FromRequest<B, C> for ($first,)
        where
            C: 'static + HSendSync,
            B: 'static + HSend,
            $first: FromRequest<B, C>,
            for<'a> $first::Future<'a>: HSend,
        {
            type Future<'a> = HBoxFuture<'a, Result<($first,), Error>>;

            fn from_request<'a>(parts: Request<B>, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {
                    let ret = $first::from_request(parts, state).await?;
                    Ok((ret,))
                })
            }
        }

        impl<C, $first> FromRequestParts<C> for ($first,)
        where
            C: 'static + HSendSync,
            $first: FromRequestParts<C>,
            for <'a> $first::Future<'a>: HSend,
        {
            type Future<'a> = HBoxFuture<'a, Result<($first,), Error>>;

            fn from_request_parts<'a>(parts: &'a mut Parts, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {
                    let ret = $first::from_request_parts(parts, state).await?;
                    Ok((ret,))
                })
            }
        }
    };
    ($first: ident, $($rest:ident),+) => {
        from_request!($($rest),+);

        impl<B, C, $first, $($rest),*> FromRequest<B, C> for ($($rest),+,$first)
        where
            C: 'static + HSendSync,
            B: 'static + HSend,
            $first: FromRequest<B, C> + HSend,
            for<'a> $first::Future<'a>: HSend,
            $(
                $rest: FromRequestParts<C> + HSend,
                for<'a> $rest::Future<'a>: HSend
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

        impl< C, $first, $($rest),*> FromRequestParts<C> for ($($rest),+,$first)
        where
            C: 'static + HSendSync,
            $first: FromRequestParts<C> + HSend,
            for<'a> $first::Future<'a>: HSend,
            $(
                $rest: FromRequestParts<C> + HSend,
                for<'a> $rest::Future<'a>: HSend
            ),+
        {
            type Future<'a> = HBoxFuture<'a, Result<($($rest),+,$first), Error>>;

            #[allow(non_snake_case, unused_parens)]
            fn from_request_parts<'a>(parts: &'a mut Parts, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {


                    let ($($rest),+) = (
                        $(
                            $rest::from_request_parts(parts, state).await?
                        ),+
                    );
                    let ret = $first::from_request_parts(parts, state).await?;
                    Ok(($($rest),+,ret))
                })
            }
        }
    };
    ($($ty: ty => $method: ident),+) => {
        $(
            impl<C> FromRequestParts<C> for $ty {
                type Future<'a> = core::future::Ready<Result<Self, Error>> where Self: 'a, C: 'a;
                fn from_request_parts<'a>(parts: &'a mut Parts, _state: &'a C) -> Self::Future<'a> {
                    core::future::ready(Ok(parts.$method.clone()))
                }
            }

            impl<B: 'static, C, > FromRequest<B, C> for $ty {
                type Future<'a> = core::future::Ready<Result<Self, Error>> where Self: 'a, C: 'a;
                fn from_request<'a>(req: Request<B>, _state: &'a C) -> Self::Future<'a> {
                    core::future::ready(Ok(req.$method().clone()))
                }
            }
        )+
    };
}

from_request!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

from_request!( Uri => uri, HeaderMap => headers);
