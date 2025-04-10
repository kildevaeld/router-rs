use std::convert::Infallible;

use wilbur_container::modules::Module;
use wilbur_routing::{RouterBuildContext, Routing};

use crate::modifier::CookiesJarModifier;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CookiesConfig {}

pub struct CookiesModule;

impl<C> Module<C> for CookiesModule
where
    C: RouterBuildContext + Routing<C::Context, C::Body>,
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
