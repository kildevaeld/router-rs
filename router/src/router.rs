use std::collections::HashMap;
use std::sync::Arc;

pub use crate::error::Error;
use crate::traits::{MaybeSend, MaybeSendSync};
use crate::{
    handler::{BoxHandler, Handler, box_handler},
    middleware::{BoxMiddleware, Middleware, box_middleware},
};
#[cfg(feature = "tower")]
use heather::{BoxFuture, Hrc};
#[cfg(feature = "tower")]
use http::{Request, Response};
use routing::Params;
use routing::router::MethodFilter;

pub struct Builder<C, B> {
    tree: routing::router::Router<BoxHandler<B, C>>,
    middlewares: Vec<BoxMiddleware<B, C, BoxHandler<B, C>>>,
}

impl<C: MaybeSendSync + 'static, B: MaybeSend + 'static> Builder<C, B> {
    pub fn new() -> Builder<C, B> {
        Builder {
            tree: routing::router::Router::new(),
            middlewares: Default::default(),
        }
    }

    pub fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), Error>
    where
        T: Handler<B, C> + 'static,
    {
        self.tree.route(method, path, box_handler(handler))?;
        Ok(())
    }

    pub fn middleware<M>(&mut self, middleware: M) -> Result<(), Error>
    where
        M: Middleware<B, C, BoxHandler<B, C>> + 'static,
    {
        self.middlewares.push(box_middleware(middleware).into());
        Ok(())
    }

    pub fn match_route<P: Params>(
        &self,
        path: &str,
        method: MethodFilter,
        params: &mut P,
    ) -> Option<&BoxHandler<B, C>> {
        self.tree.match_route(path, method, params)
    }

    #[cfg(feature = "tower")]
    pub fn into_service(self, context: C) -> RouterService<C, B> {
        let router = self.tree.map(|m| compile(&self.middlewares, m));
        RouterService {
            router: Router { tree: router }.into(),
            context,
        }
    }
}

pub struct Router<C, B> {
    tree: routing::router::Router<BoxHandler<B, C>>,
}

impl<C, B> Router<C, B> {
    pub fn match_path<P: Params>(
        &self,
        path: &str,
        method: MethodFilter,
        params: &mut P,
    ) -> Option<&BoxHandler<B, C>> {
        self.tree.match_route(path, method, params)
    }
}

#[cfg(feature = "tower")]
pub fn compile<B, C>(
    middlewares: &[BoxMiddleware<B, C, BoxHandler<B, C>>],
    task: BoxHandler<B, C>,
) -> BoxHandler<B, C> {
    let mut iter = middlewares.iter();
    let Some(middleware) = iter.next() else {
        return task;
    };

    let mut handler = middleware.wrap(task);
    while let Some(middleware) = iter.next() {
        handler = middleware.wrap(handler);
    }

    handler
}

#[cfg(feature = "tower")]
pub struct RouterService<C, B> {
    router: Hrc<Router<C, B>>,
    context: C,
}

#[cfg(feature = "tower")]
impl<C, B> tower::Service<Request<B>> for RouterService<C, B>
where
    B: MaybeSend + 'static,
    C: Clone + MaybeSendSync + 'static,
{
    type Response = Response<B>;

    type Error = Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        core::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let router = self.router.clone();
        let context = self.context.clone();
        Box::pin(async move {
            //
            let mut params = HashMap::<Arc<str>, Arc<str>>::default();
            let Some(handle) =
                router.match_path(req.uri().path(), req.method().clone().into(), &mut params)
            else {
                todo!()
            };

            handle.call(&context, req).await
        })
    }
}

#[cfg(feature = "hyper")]
impl<C, B> hyper::service::Service<Request<B>> for RouterService<C, B>
where
    B: MaybeSend + 'static,
    C: Clone + MaybeSendSync + 'static,
{
    type Response = Response<B>;

    type Error = Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&self, req: Request<B>) -> Self::Future {
        let router = self.router.clone();
        let context = self.context.clone();
        Box::pin(async move {
            //
            let mut params = HashMap::<Arc<str>, Arc<str>>::default();
            let Some(handle) =
                router.match_path(req.uri().path(), req.method().clone().into(), &mut params)
            else {
                todo!()
            };

            handle.call(&context, req).await
        })
    }
}

#[cfg(feature = "tower")]
impl<C, B> Clone for RouterService<C, B>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            context: self.context.clone(),
        }
    }
}

// pin_project! {
//     pub struct RouterServiceFuture<C, B> {
//         router: Router<C, B>,
//         content: C,
//     }
// }

// impl<C, B> Future for RouterServiceFuture<C, B> {
//     type Output = Result<Response<B>, Error>;

//     fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
//         todo!()
//     }
// }
