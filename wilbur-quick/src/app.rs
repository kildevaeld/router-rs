use std::sync::Arc;

use heather::HBoxFuture;
use rquickjs::{CatchResultExt, Class, Ctx, Function, Value};
use rquickjs_util::RuntimeError;
use wilbur_container::modules::{BuildContext, Module};

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

    pub async fn build(
        self,
        ctx: Ctx<'js>,
    ) -> rquickjs::Result<(Class<'js, Router<'js>>, JsRouteContext)> {
        let mut context = JsBuildContext::new(ctx)?;

        for module in self.modules {
            module.call(&mut context).await;
        }

        Ok(context.build().await.unwrap())
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
pub struct AppBuilder {
    inits: Vec<Arc<dyn Init + Send + Sync>>,
}

impl AppBuilder {
    pub fn add_init<I: Init + Send + Sync + 'static>(&mut self, init: I) {
        self.inits.push(Arc::from(init));
    }

    pub async fn build<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<()> {
        let mut app = App {
            modules: Default::default(),
        };
        for init in &self.inits {
            init.init(&mut app);
        }

        let (router, context) = app.build(ctx.clone()).await?;

        let router = router.borrow_mut().build();

        ctx.globals().set("Wilbur", JsApp { router, context })?;

        Ok(())
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

pub struct InitPath<S>(pub S);

impl<S> Init for InitPath<S>
where
    S: Into<Vec<u8>> + Clone + 'static,
{
    fn init<'js>(&self, app: &mut App<'js>) {
        app.add_module(InitPathModule(self.0.clone()));
    }
}

pub struct InitPathModule<S>(S);

impl<'js, S> Module<JsBuildContext<'js>> for InitPathModule<S>
where
    S: Into<Vec<u8>> + 'js,
{
    type Error = RuntimeError;

    fn build<'a>(
        self,
        ctx: &'a mut JsBuildContext<'js>,
    ) -> impl Future<Output = Result<(), Self::Error>> + heather::HSend + 'a {
        async move {
            let module = rquickjs::Module::import(&*ctx, self.0)
                .catch(&*ctx)?
                .into_future::<rquickjs::Object>()
                .await
                .catch(&*ctx)?;

            let func = module.get::<_, Function>("default").catch(&*ctx)?;

            let output = func.call::<_, Value>((ctx.router.clone(),)).catch(&*ctx)?;

            if let Some(promise) = output.as_promise() {
                promise.clone().into_future::<()>().await.catch(&*ctx)?;
            }

            Ok(())
        }
    }
}
