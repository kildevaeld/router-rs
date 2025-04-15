use heather::{HSend, Hrc};
use rquickjs::{Class, Ctx, class::Trace};
use wilbur_container::{Extensible, ExtensibleMut, Extensions, modules::BuildContext};
use wilbur_core::Error;
use wilbur_routing::{RouterBuildContext, Routing};

use crate::router::{JsHandler, Router};

pub struct JsBuildContext<'js> {
    pub(crate) router: Class<'js, Router<'js>>,
    pub(crate) extensions: Extensions,
    ctx: Ctx<'js>,
}

impl<'js> JsBuildContext<'js> {
    pub fn new(ctx: Ctx<'js>) -> rquickjs::Result<Self> {
        let router = Class::instance(ctx.clone(), Router::new())?;

        Ok(JsBuildContext {
            router,
            extensions: Default::default(),
            ctx,
        })
    }
}

impl<'js> core::ops::Deref for JsBuildContext<'js> {
    type Target = Ctx<'js>;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl Extensible for JsBuildContext<'_> {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

impl ExtensibleMut for JsBuildContext<'_> {
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

impl<'js> BuildContext for JsBuildContext<'js> {
    type Context = JsRouteContext;

    type Output = (Class<'js, Router<'js>>, Self::Context);

    type Error = Error;

    fn build(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + HSend {
        async move {
            //

            Ok((
                self.router,
                JsRouteContext {
                    extensions: self.extensions.into(),
                },
            ))
        }
    }
}

impl<'js> Routing<reggie::Body, JsRouteContext> for JsBuildContext<'js> {
    type Handler = JsHandler<'js>;

    fn modifier<M: wilbur_core::Modifier<reggie::Body, JsRouteContext> + 'static>(
        &mut self,
        modifier: M,
    ) {
        self.router.borrow_mut().modifier(modifier);
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
        self.router.borrow_mut().route(method, path, handler)
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), wilbur_routing::RouteError>
    where
        M: wilbur_core::Middleware<reggie::Body, JsRouteContext, Self::Handler> + 'static,
    {
        self.router.borrow_mut().middleware(middleware)
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
