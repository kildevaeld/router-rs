use crate::{Extensible, ExtensibleMut};
use alloc::{boxed::Box, vec::Vec};
use heather::{HBoxFuture, HSend, HSendSync};

pub trait BuildContext: ExtensibleMut {
    type Context: Extensible;
    type Output;
    type Error;

    fn build(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + HSend;
}

pub trait Module<C: BuildContext>: HSendSync {
    type Error;
    fn build<'a>(
        self,
        ctx: &'a mut C,
    ) -> impl Future<Output = Result<(), Self::Error>> + HSend + 'a;
}

pub trait DynModule<C: BuildContext>: HSendSync {
    fn build<'a>(
        self: Box<Self>,
        ctx: &'a mut C,
    ) -> HBoxFuture<'a, Result<(), Box<dyn core::error::Error + Send + Sync>>>;
}

pub type BoxModule<C> = Box<dyn DynModule<C>>;

impl<C> Module<C> for BoxModule<C>
where
    C: BuildContext,
{
    type Error = Box<dyn core::error::Error + Send + Sync>;

    fn build<'a>(
        self,
        ctx: &'a mut C,
    ) -> impl Future<Output = Result<(), Self::Error>> + HSend + 'a {
        async move { self.build(ctx).await }
    }
}

pub struct ModuleBox<T>(T);

impl<T> ModuleBox<T> {
    pub fn new<C>(module: T) -> Box<dyn DynModule<C>>
    where
        C: BuildContext,
        T: Module<C> + 'static,
        T::Error: Into<Box<dyn core::error::Error + Send + Sync>> + 'static,
    {
        Box::new(ModuleBox(module))
    }
}

impl<C, T> DynModule<C> for ModuleBox<T>
where
    C: BuildContext,
    T: Module<C> + 'static,
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>> + 'static,
{
    fn build<'a>(
        self: Box<Self>,
        ctx: &'a mut C,
    ) -> HBoxFuture<'a, Result<(), Box<dyn core::error::Error + Send + Sync>>> {
        Box::pin(async move { self.0.build(ctx).await.map_err(Into::into) })
    }
}

pub struct Builder<C: BuildContext> {
    modules: Vec<BoxModule<C>>,
}

impl<C: BuildContext> Builder<C>
where
    C::Error: From<Box<dyn core::error::Error + Send + Sync>>,
{
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn add_module<M>(&mut self, module: M)
    where
        M: Module<C> + 'static,
        M::Error: Into<Box<dyn core::error::Error + Send + Sync>> + 'static,
    {
        self.modules.push(ModuleBox::new(module));
    }

    pub fn with_module<M>(mut self, module: M) -> Self
    where
        M: Module<C> + 'static,
        M::Error: Into<Box<dyn core::error::Error + Send + Sync>> + 'static,
    {
        self.modules.push(ModuleBox::new(module));
        self
    }

    pub async fn build(self, mut ctx: C) -> Result<C::Output, C::Error> {
        for module in self.modules {
            module.build(&mut ctx).await.map_err(|e| {
                // Handle the error conversion if necessary
                e.into()
            })?;
        }
        ctx.build().await
    }
}
