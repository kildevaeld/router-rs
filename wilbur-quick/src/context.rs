use std::convert::Infallible;

use heather::{HSend, Hrc};
use rquickjs::class::Trace;
use wilbur_container::{Extensible, ExtensibleMut, Extensions, modules::BuildContext};
use wilbur_core::Error;
use wilbur_routing::{RouterBuildContext, Routing};

use crate::router::{JsHandler, Router};

pub struct JsBuildContext<'js> {
    router: Router<'js>,
    extensions: Extensions,
}

impl<'js> Default for JsBuildContext<'js> {
    fn default() -> Self {
        JsBuildContext {
            router: Router::new(),
            extensions: Default::default(),
        }
    }
}

impl<'js> Extensible for JsBuildContext<'js> {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

impl<'js> ExtensibleMut for JsBuildContext<'js> {
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

impl<'js> BuildContext for JsBuildContext<'js> {
    type Context = JsRouteContext;

    type Output = (Router<'js>, Self::Context);

    type Error = Error;

    fn build(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + HSend {
        async move { todo!() }
    }
}

impl<'js> Routing<reggie::Body, JsRouteContext> for JsBuildContext<'js> {
    type Handler = JsHandler<'js>;

    fn modifier<M: wilbur_core::Modifier<reggie::Body, JsRouteContext> + 'static>(
        &mut self,
        modifier: M,
    ) {
        todo!()
    }

    fn route<T>(
        &mut self,
        method: wilbur_routing::MethodFilter,
        path: &str,
        handler: T,
    ) -> Result<(), wilbur_routing::RouteError>
    where
        T: wilbur_core::Handler<reggie::Body, JsRouteContext> + 'static,
    {
        todo!()
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), wilbur_routing::RouteError>
    where
        M: wilbur_core::Middleware<reggie::Body, JsRouteContext, Self::Handler> + 'static,
    {
        todo!()
    }
}

impl<'js> RouterBuildContext for JsBuildContext<'js> {
    type Body = reggie::Body;
}

#[derive(Debug, Clone, Default)]
pub struct JsRouteContext {
    extensions: Hrc<Extensions>,
}

impl Extensible for JsRouteContext {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

impl<'js> Trace<'js> for JsRouteContext {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {}
}
