use core::marker::PhantomData;
use heather::{HBoxFuture, HSend, HSendSync, Hrc};
use http::{Request, Response};

use crate::into_response::IntoResponse;

pub trait Handler<B, C>: HSendSync {
    type Response: IntoResponse<B>;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Response, Self::Error>> + HSend
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a>;
}

// pub trait DynHandler<B, C>: HSendSync {
//     fn call<'a>(
//         &'a self,
//         context: &'a C,
//         req: Request<B>,
//     ) -> HBoxFuture<'a, Result<Response<B>, Error>>;
// }

// pub fn box_handler<C, B, T>(handler: T) -> BoxHandler<B, C>
// where
//     T: Handler<B, C> + 'static,
//     B: HSend + 'static,
//     C: HSendSync + 'static,
// {
//     BoxHandler {
//         inner: Hrc::from(HandlerBox(handler, PhantomData, PhantomData)),
//     }
// }

// pub struct HandlerBox<B, C, T>(T, PhantomData<C>, PhantomData<B>);

// unsafe impl<B, C, T: Send> Send for HandlerBox<B, C, T> {}

// unsafe impl<B, C, T: Sync> Sync for HandlerBox<B, C, T> {}

// impl<B, C, T> DynHandler<B, C> for HandlerBox<B, C, T>
// where
//     T: Handler<B, C> + 'static,
//     C: HSendSync + 'static,
//     B: HSend,
// {
//     fn call<'a>(
//         &'a self,
//         context: &'a C,
//         req: Request<B>,
//     ) -> HBoxFuture<'a, Result<Response<B>, Error>> {
//         Box::pin(async move { self.0.call(context, req).await.map(|m| m.into_response()) })
//     }
// }

// pub struct BoxHandler<B, C> {
//     inner: Hrc<dyn DynHandler<B, C>>,
// }

// unsafe impl<B, C> Send for BoxHandler<B, C> where Hrc<dyn DynHandler<B, C>>: Send {}

// unsafe impl<B, C> Sync for BoxHandler<B, C> where Hrc<dyn DynHandler<B, C>>: Sync {}

// impl<B, C> Handler<B, C> for BoxHandler<B, C> {
//     type Response = Response<B>;

//     type Future<'a>
//         = HBoxFuture<'a, Result<Self::Response, Error>>
//     where
//         Self: 'a,
//         C: 'a;

//     fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a> {
//         self.inner.call(context, req)
//     }
// }

// impl<B, C> Clone for BoxHandler<B, C> {
//     fn clone(&self) -> Self {
//         Self {
//             inner: self.inner.clone(),
//         }
//     }
// }
