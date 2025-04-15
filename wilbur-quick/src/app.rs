use std::sync::Arc;

use heather::HBoxFuture;
use rquickjs::{Class, Ctx};
use rquickjs_util::RuntimeError;
use wilbur_container::modules::{BoxModule, BuildContext, Builder, Module};

use crate::{
    JsApp,
    context::{JsBuildContext, JsRouteContext},
    router::Router,
};

pub trait ModuleInit<'js> {
    fn call<'a>(
        self: Box<Self>,
        ctx: &'a mut JsBuildContext<'js>,
    ) -> HBoxFuture<'a, Result<(), RuntimeError>>;
}

struct Wrapper<T>(T);

impl<'js, T> ModuleInit<'js> for Wrapper<T>
where
    T: Module<JsBuildContext<'js>> + 'js,
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    fn call<'a>(
        self: Box<Self>,
        ctx: &'a mut JsBuildContext<'js>,
    ) -> HBoxFuture<'a, Result<(), RuntimeError>> {
        Box::pin(async move {
            self.0
                .build(ctx)
                .await
                .map_err(|err| RuntimeError::Custom(err.into()))?;
            Ok(())
        })
    }
}

#[derive(Default)]
pub struct App<'js> {
    modules: Vec<Box<dyn ModuleInit<'js> + 'js>>,
}

impl<'js> App<'js> {
    pub fn add_module<T: Module<JsBuildContext<'js>> + 'js>(&mut self, module: T)
    where
        T: 'js,
        T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        // self.modules.push(Box::new(module));
        self.modules.push(Box::new(Wrapper(module)));
    }

    pub async fn build(self) -> (Router<'js>, JsRouteContext) {
        let mut context = JsBuildContext::default();

        for module in self.modules {
            module.call(&mut context).await;
        }

        context.build().await.unwrap()
    }
}

pub trait Init {
    fn init<'js>(&self, app: &mut App<'js>);
}

impl<T> Init for T
where
    for<'a> T: Fn(&mut App<'a>),
{
    fn init<'js>(&self, app: &mut App<'js>) {
        (self)(app)
    }
}

#[derive(Default, Clone)]
pub struct InitList {
    inits: Vec<Arc<dyn Init + Send + Sync>>,
}

impl InitList {
    pub fn add_init<I: Init + Send + Sync + 'static>(&mut self, init: I) {
        self.inits.push(Arc::from(init));
    }

    pub async fn build<'js>(&self, ctx: Ctx<'js>) {
        let mut app = App::default();
        for init in &self.inits {
            init.init(&mut app);
        }

        let (router, context) = app.build().await;

        let router = router.build();

        ctx.globals().set("Wilbur", JsApp { router, context });
    }
}

pub struct InitModule<T>(pub T);

impl<T> Init for InitModule<T>
where
    T: Clone + 'static,
    for<'js> <T as Module<JsBuildContext<'js>>>::Error:
        Into<Box<dyn core::error::Error + Send + Sync>>,
    for<'js> T: Module<JsBuildContext<'js>>,
{
    fn init<'js>(&self, app: &mut App<'js>) {
        app.add_module(self.0.clone());
    }
}
