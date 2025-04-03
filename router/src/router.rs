use std::collections::HashMap;
use std::sync::Arc;

pub use crate::error::Error;
use crate::modifier::{BoxModifier, Modifier, ModifierList, Modify, modifier_box};
use crate::traits::{MaybeSend, MaybeSendSync, Routing};
use crate::{
    handler::{BoxHandler, Handler, box_handler},
    middleware::{BoxMiddleware, Middleware, box_middleware},
};
#[cfg(any(feature = "tower", feature = "hyper"))]
use heather::BoxFuture;
use heather::Hrc;
use http::{Request, Response};
use routing::Params;
use routing::router::MethodFilter;

pub struct Builder<C, B> {
    tree: routing::router::Router<BoxHandler<B, C>>,
    middlewares: Vec<BoxMiddleware<B, C, BoxHandler<B, C>>>,
    modifiers: Vec<BoxModifier<B, C>>,
}

impl<C: MaybeSendSync + 'static, B: MaybeSend + 'static> Builder<C, B> {
    pub fn new() -> Builder<C, B> {
        Builder {
            tree: routing::router::Router::new(),
            middlewares: Default::default(),
            modifiers: Default::default(),
        }
    }

    pub fn modifier<M: Modifier<B, C> + 'static>(&mut self, modifier: M) {
        self.modifiers.push(modifier_box(modifier));
    }

    pub fn mount(&mut self, path: &str, router: impl Into<Router<C, B>>) {
        let router = router.into();
        self.tree.mount(path, router.tree);
    }

    pub fn merge(&mut self, router: impl Into<Router<C, B>>) {
        let router = router.into();
        self.tree.merge(router.tree);
    }

    // pub fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), Error>
    // where
    //     T: Handler<B, C> + 'static,
    // {
    //     self.tree.route(method, path, box_handler(handler))?;
    //     Ok(())
    // }

    // pub fn middleware<M>(&mut self, middleware: M) -> Result<(), Error>
    // where
    //     M: Middleware<B, C, BoxHandler<B, C>> + 'static,
    // {
    //     self.middlewares.push(box_middleware(middleware).into());
    //     Ok(())
    // }

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
        RouterService {
            router: Hrc::new(self.into()),
            context,
        }
    }
}

impl<C: MaybeSendSync + 'static, B: MaybeSend + 'static> Routing<C, B> for Builder<C, B> {
    type Handler = BoxHandler<B, C>;

    fn modifier<M: Modifier<B, C> + 'static>(&mut self, modifier: M) {
        self.modifiers.push(modifier_box(modifier));
    }

    fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), Error>
    where
        T: Handler<B, C> + 'static,
    {
        self.tree.route(method, path, box_handler(handler))?;
        Ok(())
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), Error>
    where
        M: Middleware<B, C, Self::Handler> + 'static,
    {
        self.middlewares.push(box_middleware(middleware));
        Ok(())
    }
}

impl<C, B> From<Builder<C, B>> for Router<C, B> {
    fn from(value: Builder<C, B>) -> Self {
        let tree = value.tree.map(|m| compile(&value.middlewares, m));
        let modifiers: Hrc<[BoxModifier<B, C>]> = value.modifiers.into();
        Router {
            tree,
            modifiers: modifiers.into(),
        }
    }
}

pub struct Router<C, B> {
    tree: routing::router::Router<BoxHandler<B, C>>,
    modifiers: ModifierList<B, C>,
}

impl<C: MaybeSendSync, B: MaybeSend> Router<C, B> {
    pub fn match_path<P: Params>(
        &self,
        path: &str,
        method: MethodFilter,
        params: &mut P,
    ) -> Option<&BoxHandler<B, C>> {
        self.tree.match_route(path, method, params)
    }

    pub async fn handle(&self, mut req: Request<B>, context: &C) -> Result<Response<B>, Error> {
        //
        let mut params = HashMap::<Arc<str>, Arc<str>>::default();
        let Some(handle) =
            self.match_path(req.uri().path(), req.method().clone().into(), &mut params)
        else {
            todo!()
        };

        req.extensions_mut().insert(UrlParams { inner: params });

        let modify = self.modifiers.before(&mut req, context).await;

        let mut resp = handle.call(context, req).await?;

        modify.modify(&mut resp, context).await;

        Ok(resp)
    }

    pub fn into_parts(
        self,
    ) -> (
        routing::router::Router<BoxHandler<B, C>>,
        ModifierList<B, C>,
    ) {
        (self.tree, self.modifiers)
    }
}

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

#[cfg(any(feature = "tower", feature = "hyper"))]
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
        let this = self.clone();
        Box::pin(async move { this.router.handle(req, &this.context).await })
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
        let this = self.clone();
        Box::pin(async move { this.router.handle(req, &this.context).await })
    }
}

#[cfg(any(feature = "tower", feature = "hyper"))]
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

#[derive(Debug, Clone)]
pub struct UrlParams {
    inner: HashMap<Arc<str>, Arc<str>>,
}

impl UrlParams {
    pub fn get(&self, name: &str) -> Option<&Arc<str>> {
        self.inner.get(name)
    }
}
