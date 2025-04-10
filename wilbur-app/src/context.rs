use heather::{HSend, Hrc};
use wilbur_container::{Extensible, ExtensibleMut, Extensions, modules::BuildContext};
use wilbur_core::{Handler, Middleware, Modifier};
use wilbur_routing::{Builder, MethodFilter, RouteError, Router, RouterBuildContext, Routing};

use crate::body::Body;

#[derive(Clone)]
pub struct Context {
    extensions: Hrc<Extensions>,
}

impl Extensible for Context {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

pub struct RouterContext {
    extensions: Extensions,
    router: Builder<Body, Context>,
}

impl RouterContext {
    pub fn new() -> RouterContext {
        RouterContext {
            extensions: Default::default(),
            router: Builder::new(),
        }
    }
}

impl Extensible for RouterContext {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

impl ExtensibleMut for RouterContext {
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

impl Routing<Body, Context> for RouterContext {
    type Handler = <Builder<Body, Context> as Routing<Body, Context>>::Handler;

    fn modifier<M: Modifier<Body, Context> + 'static>(&mut self, modifier: M) {
        self.router.modifier(modifier);
    }

    fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<Body, Context> + 'static,
    {
        self.router.route(method, path, handler)
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), RouteError>
    where
        M: Middleware<Body, Context, Self::Handler> + 'static,
    {
        self.router.middleware(middleware)
    }

    // fn merge(&mut self, router: Self) -> Result<(), RouteError> {
    //     self.router.merge(router.router)?;
    //     Ok(())
    // }

    // fn mount<T: Into<Self>>(&mut self, path: &str, router: T) -> Result<(), RouteError> {
    //     self.router.mount(path, router.into().router)?;
    //     Ok(())
    // }
}

impl BuildContext for RouterContext {
    type Context = Context;
    type Output = (Router<Body, Context>, Context);
    type Error = wilbur_core::Error;

    fn build(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + HSend {
        async move {
            Ok((
                self.router.into(),
                Context {
                    extensions: self.extensions.into(),
                },
            ))
        }
    }
}

impl RouterBuildContext for RouterContext {
    type Body = Body;
}
