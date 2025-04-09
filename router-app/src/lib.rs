use context::RouterContext;
use heather::HBoxError;
use uhuh_container::modules::Builder;

mod body;
mod context;
mod error;

pub struct App {
    builder: Builder<RouterContext>,
}

impl App {
    pub fn add_module<M>(&mut self, module: M)
    where
        M: uhuh_container::modules::Module<RouterContext> + 'static,
        M::Error: Into<HBoxError<'static>>,
    {
        self.builder.add_module(module);
    }

    pub async fn build(self) -> Result<(), error::Error> {
        let (router, context) = self.builder.build(RouterContext::new()).await.unwrap();

        Ok(())
    }
}
