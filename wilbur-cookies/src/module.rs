use std::convert::Infallible;

use heather::HSend;
use wilbur_container::modules::Module;
use wilbur_routing::{RouterBuildContext, Routing};

use crate::modifier::CookiesJarModifier;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CookiesConfig {}

#[derive(Debug, Clone, Copy)]
pub struct CookiesModule;

impl<C> Module<C> for CookiesModule
where
    C: RouterBuildContext + Routing<C::Body, C::Context> + HSend,
    C::Body: HSend,
{
    type Error = Infallible;

    fn build<'a>(
        self,
        ctx: &'a mut C,
    ) -> impl Future<Output = Result<(), Self::Error>> + heather::HSend + 'a {
        async move {
            ctx.modifier(CookiesJarModifier {});

            Ok(())
        }
    }
}
