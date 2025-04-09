use heather::{HSend, HSendSync, Hrc};
use router::{Builder, MethodFilter, Router, Routing};
use uhuh_container::{Extensible, ExtensibleMut, Extensions, modules::BuildContext};

use crate::body::Body;

pub struct Context {
    extensions: Hrc<Extensions>,
}

fn test<T: HSendSync>() {}

fn rap() {
    test::<Context>()
}

impl Extensible for Context {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

pub struct RouterContext {
    extensions: Extensions,
    router: Builder<Context, Body>,
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

impl Routing<Context, Body> for RouterContext {
    type Handler = <Builder<Context, Body> as Routing<Context, Body>>::Handler;

    fn modifier<M: router::Modifier<Body, Context> + 'static>(&mut self, modifier: M) {
        self.router.modifier(modifier);
    }

    fn route<T>(
        &mut self,
        method: MethodFilter,
        path: &str,
        handler: T,
    ) -> Result<(), router::Error>
    where
        T: router::Handler<Body, Context> + 'static,
    {
        self.router.route(method, path, handler)
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), router::Error>
    where
        M: router::Middleware<Body, Context, Self::Handler> + 'static,
    {
        self.router.middleware(middleware)
    }
}

impl BuildContext for RouterContext {
    type Context = Context;
    type Output = (Router<Context, Body>, Context);
    type Error = router::Error;

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
