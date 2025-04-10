use std::collections::HashMap;
use std::sync::Arc;

pub use crate::error::Error;

use crate::Routing;
use crate::params::UrlParams;
#[cfg(any(feature = "tower", feature = "hyper"))]
use crate::service::RouterService;
use heather::{HSend, HSendSync, Hrc};
use hyper::{Request, Response};
use routing::Params;
use routing::router::MethodFilter;
use wilbur_core::handler::{BoxHandler, box_handler};
use wilbur_core::middleware::{BoxMiddleware, box_middleware};
use wilbur_core::modifier::{BoxModifier, ModifierList, modifier_box};
use wilbur_core::{Handler, Middleware, Modifier, Modify};

pub struct Builder<B, C> {
    tree: routing::router::Router<BoxHandler<B, C>>,
    middlewares: Vec<BoxMiddleware<B, C, BoxHandler<B, C>>>,
    modifiers: Vec<BoxModifier<B, C>>,
}

impl<B: HSend + 'static, C: HSendSync + 'static> Builder<B, C> {
    pub fn new() -> Builder<B, C> {
        Builder {
            tree: routing::router::Router::new(),
            middlewares: Default::default(),
            modifiers: Default::default(),
        }
    }

    pub fn mount(&mut self, path: &str, router: impl Into<Router<B, C>>) -> Result<(), Error> {
        let router = router.into();
        self.tree.mount(path, router.tree)?;
        Ok(())
    }

    pub fn merge(&mut self, router: impl Into<Router<B, C>>) -> Result<(), Error> {
        let router = router.into();
        self.tree.merge(router.tree)?;
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

    #[cfg(any(feature = "tower", feature = "hyper"))]
    pub fn into_service(self, context: C) -> RouterService<B, C> {
        RouterService {
            router: Hrc::new(self.into()),
            context,
        }
    }
}

impl<B: HSend + 'static, C: HSendSync + 'static> Routing<B, C> for Builder<B, C> {
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

    fn merge(&mut self, router: Self) -> Result<(), Error> {
        Ok(())
    }

    fn mount(&mut self, path: &str, router: Self) -> Result<(), Error> {
        Ok(())
    }
}

impl<B, C> From<Builder<B, C>> for Router<B, C> {
    fn from(value: Builder<B, C>) -> Self {
        let tree = value.tree.map(|m| compose(&value.middlewares, m));
        let modifiers: Hrc<[BoxModifier<B, C>]> = value.modifiers.into();
        Router {
            tree,
            modifiers: modifiers.into(),
        }
    }
}

pub struct Router<B, C> {
    tree: routing::router::Router<BoxHandler<B, C>>,
    modifiers: ModifierList<B, C>,
}

impl<B: HSend, C: HSendSync> Router<B, C> {
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

    #[cfg(any(feature = "tower", feature = "hyper"))]
    pub fn into_service(self, context: C) -> RouterService<B, C> {
        RouterService {
            router: Hrc::new(self),
            context,
        }
    }
}

pub fn compose<B, C>(
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
