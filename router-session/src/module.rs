use router::{RouterBuildContext, Routing};
use uhuh_container::modules::Module;
use uhuh_container::prelude::*;

#[cfg(feature = "quick")]
use crate::bindings;
use crate::{modifier::SessionModifier, session_store::SessionStore};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionModule {
    #[serde(default)]
    pub anonymous: bool,
}

impl<C: RouterBuildContext> Module<C> for SessionModule
where
    C: RouterBuildContext + Routing<C::Context, C::Body>,
{
    type Error = std::convert::Infallible;

    fn build<'a>(
        self,
        ctx: &'a mut C,
    ) -> impl Future<Output = Result<(), Self::Error>> + heather::HSend + 'a {
        async move {
            let session_store = SessionStore::default();

            ctx.register(session_store);
            ctx.modifier(SessionModifier::default());

            Ok(())
        }
    }
}

/*impl<C: ExtensibleContext + 'static> uhuh_core::Application<C> for SessionApp
where
    C::Output: RunContext + Send + Sync + Clone + 'static,
{
    const NAME: &'static str = "session";

    type Config = SessionConfig;
    type State = ();
    type Error = CoreError;

    fn default_config() -> Option<Self::Config> {
        Some(SessionConfig { anonymous: false })
    }

    fn setup<'a>(
        mut ctx: SetupCtx<'a, C>,
    ) -> impl std::future::Future<Output = Result<Self::State, Self::Error>> + 'a {
        async move {
            ctx.add_dependency(Dep::<wilbur_cookies::CookiesApp>::new());
            Ok(())
        }
    }
    fn build<'a>(
        mut ctx: BuildCtx<'a, C>,
        _state: &'a mut Self::State,
        _config: Option<Self::Config>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + 'a {
        async move {
            #[cfg(feature = "quick")]
            if let Some(ext) = ctx.context_mut().get_mut::<uhuh_quick::QuickBuilder<C>>() {
                struct Ext;

                impl<C: RunContext + Send + Sync + Clone + 'static>
                    uhuh_quick::Augmentation<C, Request> for Ext
                {
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
                                "../wilbur-session.d.ts"
                            ))))
                        }
                    }

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
                            let session: Option<Session> =
                                request.extract_parts_with_state(wilbur).await.ok();

                            if let Some(session) = session {
                                obj.register("session", move |ctx| {
                                    let ret = rquickjs::Class::instance(
                                        ctx,
                                        bindings::JsSession { session },
                                    )?;
                                    Ok(ret.into_value())
                                });
                            }

                            Ok(())
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
            let store = SessionStore::default();
            ctx.register(store);
            ctx.add_modifier(SessionModifier::default());

            Ok(())
        }
    }
}
*/
