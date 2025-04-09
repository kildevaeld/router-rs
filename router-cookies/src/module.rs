use std::convert::Infallible;

use router::{RouterBuildContext, Routing};
#[cfg(feature = "quick")]
use rquickjs_util::RuntimeError;
use uhuh_container::modules::Module;

#[cfg(feature = "quick")]
use crate::bindings::JsCookieJar;
use crate::{CookieJar, modifier::CookiesJarModifier};

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

/*
impl<C: ExtensibleContext + 'static> uhuh_core::Application<C> for CookiesApp
where
    C::Output: Send + Sync + Clone + 'static,
{
    const NAME: &'static str = "cookies";

    type Config = CookiesConfig;
    type State = ();
    type Error = CoreError;

    fn default_config() -> Option<Self::Config> {
        Some(CookiesConfig::default())
    }

    fn setup<'a>(
        _ctx: SetupCtx<'a, C>,
    ) -> impl std::future::Future<Output = Result<Self::State, Self::Error>> + 'a {
        async move { Ok(()) }
    }

    fn build<'a>(
        mut ctx: BuildCtx<'a, C>,
        _state: &'a mut (),
        _config: Option<Self::Config>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + 'a {
        async move {
            #[cfg(feature = "quick")]
            if let Some(ext) = ctx.context_mut().get_mut::<uhuh_quick::QuickBuilder<C>>() {
                struct Ext;

                impl<C: Send + Sync + Clone + 'static> uhuh_quick::Augmentation<C, Request> for Ext {
                    fn apply_augmentation<'a, 'js: 'a>(
                        &'a self,
                        _ctx: rquickjs::Ctx<'js>,
                        obj: &'a mut uhuh_quick::Augment<'js>,
                        request: &'a mut Request,
                        wilbur: &'a C,
                    ) -> impl std::future::Future<
                        Output = Result<(), uhuh_quick::klaver::RuntimeError>,
                    > + 'a {
                        async move {
                            let cookies: CookieJar = request
                                .extract_parts_with_state(wilbur)
                                .await
                                .map_err(|_| {
                                    RuntimeError::Message(Some("cookie jar".to_string()))
                                })?;

                            obj.register("cookies", move |ctx| {
                                let ret = rquickjs::Class::instance(ctx, JsCookieJar { cookies })?;
                                Ok(ret.into_value())
                            });

                            Ok(())
                        }
                    }

                    fn typings<'a>(
                        &'a self,
                        _core: &'a C,
                    ) -> impl std::future::Future<
                        Output = Result<
                            Option<std::borrow::Cow<'static, str>>,
                            uhuh_quick::klaver::RuntimeError,
                        >,
                    > + 'a {
                        async move {
                            Ok(Some(std::borrow::Cow::Borrowed(include_str!(
                                "../cookies.d.ts"
                            ))))
                        }
                    }
                }

                ext.register_augmentation(Ext);
            }
            Ok(())
        }
    }

    fn init<'a>(
        mut ctx: InitCtx<'a, C>,
        _state: Self::State,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> {
        async move {
            ctx.add_modifier(CookiesJarModifier {});
            Ok(())
        }
    }
}
*/
