use std::{collections::HashMap, pin::Pin, rc::Rc};

use http::{Request, Response};
use http_body_util::BodyExt;
use reggie::RequestExt;
use routing::{Params, PathRouter};
use rquickjs::{
    CatchResultExt, Class, Ctx, FromJs, Function, IntoJs, JsLifetime, Value, class::Trace,
    prelude::Func,
};
use rquickjs_util::RuntimeError;
use rquickjs_util::{StringRef, throw_if};
use wilbur_core::handler::{BoxHandler, box_handler};
use wilbur_core::middleware::BoxMiddleware;
use wilbur_core::modifier::{BoxModifier, ModifierList};
use wilbur_core::{Error, Handler, Middleware, Modifier, Modify};
use wilbur_routing::{MethodFilter, Routing};

#[derive(Debug, Clone)]
pub struct JsRouteContext {}

impl<'js> IntoJs<'js> for JsRouteContext {
    fn into_js(self, ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        todo!()
    }
}

#[derive(Clone)]
pub enum JsHandler<'js> {
    Script(Function<'js>),
    Handler(BoxHandler<reggie::Body, JsRouteContext>),
    ScriptMiddleware(Function<'js>, Box<JsHandler<'js>>),
}

impl<'js> Trace<'js> for JsHandler<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        match self {
            Self::Script(script) => script.trace(tracer),
            Self::ScriptMiddleware(func, _) => func.trace(tracer),
            _ => {}
        }
    }
}

impl<'js> JsHandler<'js> {
    pub async fn call(
        &self,
        req: Request<reggie::Body>,
        context: JsRouteContext,
    ) -> Result<Response<reggie::Body>, RuntimeError> {
        match self {
            Self::Script(script) => {
                let req = klaver_wintercg::http::Request::from_request(script.ctx(), req)
                    .catch(script.ctx())?;
                let mut ret = script.call::<_, Value>((req,)).catch(script.ctx())?;
                if let Some(promise) = ret.as_promise() {
                    ret = promise
                        .clone()
                        .into_future::<Value>()
                        .await
                        .catch(script.ctx())?;
                }

                let ret = Class::<klaver_wintercg::http::Response>::from_js(script.ctx(), ret)
                    .catch(script.ctx())?;

                Ok(ret
                    .borrow_mut()
                    .to_reggie(script.ctx().clone())
                    .await
                    .catch(script.ctx())?)
            }
            Self::Handler(handler) => {
                let ret = handler
                    .call(&context, req)
                    .await
                    .map_err(|err| RuntimeError::Custom(Box::new(err)))?;
                Ok(ret)
            }
            Self::ScriptMiddleware(func, handler) => {
                let req = klaver_wintercg::http::Request::from_request(func.ctx(), req)
                    .catch(func.ctx())?;

                let mut ret = func
                    .call::<_, Value>((
                        req,
                        NextFunc {
                            handler: *handler.clone(),
                        },
                    ))
                    .catch(func.ctx())?;
                if let Some(promise) = ret.as_promise() {
                    ret = promise
                        .clone()
                        .into_future::<Value>()
                        .await
                        .catch(func.ctx())?;
                }

                let ret = Class::<klaver_wintercg::http::Response>::from_js(func.ctx(), ret)
                    .catch(func.ctx())?;

                Ok(ret
                    .borrow_mut()
                    .to_reggie(func.ctx().clone())
                    .await
                    .catch(func.ctx())?)
            }
        }
    }

    pub async fn call_js(
        &self,
        ctx: Ctx<'js>,
        req: Class<'js, klaver_wintercg::http::Request<'js>>,
        context: JsRouteContext,
    ) -> rquickjs::Result<Class<'js, klaver_wintercg::http::Response<'js>>> {
        match self {
            Self::Script(script) => {
                let mut ret = script.call::<_, Value>((req,))?;
                if let Some(promise) = ret.as_promise() {
                    ret = promise.clone().into_future::<Value>().await?;
                }

                let ret = Class::<klaver_wintercg::http::Response>::from_js(script.ctx(), ret)?;

                Ok(ret)
            }
            Self::Handler(handler) => {
                let (req, _) = req.borrow_mut().into_request(ctx.clone()).await?;

                let ret = throw_if!(ctx, handler.call(&context, req).await);
                Class::instance(
                    ctx.clone(),
                    klaver_wintercg::http::Response::from_response(ctx, "", ret)?,
                )
            }
            Self::ScriptMiddleware(func, handler) => {
                let mut ret = func.call::<_, Value>((
                    req,
                    NextFunc {
                        handler: *handler.clone(),
                    },
                ))?;
                if let Some(promise) = ret.as_promise() {
                    ret = promise.clone().into_future::<Value>().await?;
                }

                let ret = Class::<klaver_wintercg::http::Response>::from_js(&ctx, ret)?;

                Ok(ret)
            }
        }
    }
}

impl<'js> Handler<reggie::Body, JsRouteContext> for JsHandler<'js> {
    type Response = Response<reggie::Body>;

    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Error>> + 'a>>
    where
        Self: 'a,
        JsRouteContext: 'a;

    fn call<'a>(
        &'a self,
        context: &'a JsRouteContext,
        req: Request<reggie::Body>,
    ) -> Self::Future<'a> {
        Box::pin(async move { self.call(req, context.clone()).await.map_err(Error::new) })
    }
}

//
#[derive(Clone)]
pub enum JsMiddleware<'js> {
    Script(Function<'js>),
    Middleware(BoxMiddleware<reggie::Body, JsRouteContext, JsHandler<'js>>),
}

impl<'js> Trace<'js> for JsMiddleware<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        match self {
            Self::Script(script) => script.trace(tracer),
            _ => {}
        }
    }
}

// impl<'js> Middleware<'js> {
//     pub async fn call(
//         &self,
//         ctx: Ctx<'js>,
//         req: Request<reggie::Body>,
//         context: JsRouteContext,
//         handler: Handler<'js>,
//     ) -> rquickjs::Result<Response<reggie::Body>> {
//         match self {
//             Self::Script(script) => {
//                 let req = klaver_wintercg::http::Request::from_request(&ctx, req)?;

//                 let next = NextFunc { handler };
//                 let mut ret = script.call::<_, Value>((req, next))?;
//                 if let Some(promise) = ret.as_promise() {
//                     ret = promise.clone().into_future::<Value>().await?;
//                 }

//                 let ret = Class::<klaver_wintercg::http::Response>::from_js(&ctx, ret)?;

//                 ret.borrow_mut().to_reggie(ctx).await
//             }
//             Self::Middleware(middleware) => {
//                 let wrapped = middleware.clone().wrap(handler);

//                 let ret = throw_if!(ctx, wrapped.call(&context, req).await);
//                 Ok(ret)
//             }
//         }
//     }
// }

impl<'js> Middleware<reggie::Body, JsRouteContext, JsHandler<'js>> for JsMiddleware<'js> {
    type Handle = JsHandler<'js>;

    fn wrap(&self, handle: JsHandler<'js>) -> Self::Handle {
        match self {
            JsMiddleware::Middleware(middleware) => JsHandler::Handler(middleware.wrap(handle)),
            JsMiddleware::Script(script) => {
                JsHandler::ScriptMiddleware(script.clone(), Box::new(handle))
            }
        }
    }
}

struct RouteEntry<'js> {
    method: MethodFilter,
    handler: JsHandler<'js>,
}

impl<'js> Trace<'js> for RouteEntry<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        self.handler.trace(tracer);
    }
}

struct RouteHandler<'js> {
    entries: Vec<RouteEntry<'js>>,
}

impl<'js> Trace<'js> for RouteHandler<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        self.entries.trace(tracer);
    }
}

#[rquickjs::class]
pub struct Router<'js> {
    tree: routing::router::Router<JsHandler<'js>>,
    middlewares: Vec<JsMiddleware<'js>>,
    modifiers: Vec<BoxModifier<reggie::Body, JsRouteContext>>,
}

unsafe impl<'js> JsLifetime<'js> for Router<'js> {
    type Changed<'to> = Router<'to>;
}

impl<'js> Trace<'js> for Router<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        for (_, i) in self.tree.iter() {
            for entry in &i.entries {
                entry.handler.trace(tracer);
            }
        }
    }
}

impl<'js> Router<'js> {
    fn route_inner(
        &mut self,
        ctx: Ctx<'js>,
        method: MethodFilter,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        throw_if!(
            ctx,
            self.tree
                .route(method, path.as_str(), JsHandler::Script(handler))
        );

        Ok(())
    }

    // pub fn from_router(
    //     router: impl Into<router::Router<JsRouteContext, reggie::Body>>,
    // ) -> Router<'js> {
    //     let (tree, modifiers) = router.into().into_parts();

    //     let tree = tree.map(JsHandler::Handler);

    //     Router {
    //         tree,
    //         middlewares: Default::default(),
    //         modifiers: modifiers.into(),
    //     }
    // }

    pub fn match_route<P: Params>(
        &self,
        path: &str,
        method: MethodFilter,
        params: &mut P,
    ) -> Option<&JsHandler<'js>> {
        self.tree.match_route(path, method, params)
    }

    pub fn build(&self) -> ResolvedRouter<'js> {
        let tree = self
            .tree
            .clone()
            .map(|handler| compose(&self.middlewares, handler));

        ResolvedRouter {
            tree: tree.into(),
            debug: false,
            modifiers: self.modifiers.clone().into(),
        }
    }
}

#[rquickjs::methods]
impl<'js> Router<'js> {
    #[qjs(constructor)]
    fn new() -> Router<'js> {
        Router {
            tree: routing::router::Router::new(),
            middlewares: Vec::default(),
            modifiers: Default::default(),
        }
    }

    fn route(
        &mut self,
        ctx: Ctx<'js>,
        method: StringRef<'js>,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        let method: MethodFilter = throw_if!(ctx, method.as_str().parse());
        self.route_inner(ctx, method, path, handler)?;
        Ok(())
    }

    fn get(
        &mut self,
        ctx: Ctx<'js>,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        self.route_inner(ctx, MethodFilter::GET, path, handler)
    }

    fn post(
        &mut self,
        ctx: Ctx<'js>,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        self.route_inner(ctx, MethodFilter::POST, path, handler)
    }

    fn patch(
        &mut self,
        ctx: Ctx<'js>,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        self.route_inner(ctx, MethodFilter::PATCH, path, handler)
    }

    fn put(
        &mut self,
        ctx: Ctx<'js>,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        self.route_inner(ctx, MethodFilter::PUT, path, handler)
    }

    fn delete(
        &mut self,
        ctx: Ctx<'js>,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        self.route_inner(ctx, MethodFilter::DELETE, path, handler)
    }

    fn any(
        &mut self,
        ctx: Ctx<'js>,
        path: StringRef<'js>,
        handler: Function<'js>,
    ) -> rquickjs::Result<()> {
        self.route_inner(ctx, MethodFilter::all(), path, handler)
    }

    #[qjs(rename = "use")]
    fn middleware(&mut self, handler: Function<'js>) -> rquickjs::Result<()> {
        self.middlewares.push(JsMiddleware::Script(handler));
        Ok(())
    }
}

impl<'js> Routing<reggie::Body, JsRouteContext> for Router<'js> {
    type Handler = JsHandler<'js>;

    fn modifier<M: Modifier<reggie::Body, JsRouteContext> + 'static>(&mut self, modifier: M) {
        todo!()
    }

    fn route<T>(
        &mut self,
        method: MethodFilter,
        path: &str,
        handler: T,
    ) -> Result<(), wilbur_routing::RouteError>
    where
        T: Handler<reggie::Body, JsRouteContext> + 'static,
    {
        self.tree
            .route(method, path, JsHandler::Handler(box_handler(handler)))?;
        Ok(())
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), wilbur_routing::RouteError>
    where
        M: Middleware<reggie::Body, JsRouteContext, Self::Handler> + 'static,
    {
        todo!()
    }
}

#[derive(Trace)]
#[rquickjs::class]
pub struct NextFunc<'js> {
    handler: JsHandler<'js>,
}

unsafe impl<'js> JsLifetime<'js> for NextFunc<'js> {
    type Changed<'to> = NextFunc<'to>;
}

#[rquickjs::methods]
impl<'js> NextFunc<'js> {
    async fn call(
        &self,
        ctx: Ctx<'js>,
        req: Class<'js, klaver_wintercg::http::Request<'js>>,
    ) -> rquickjs::Result<Class<'js, klaver_wintercg::http::Response<'js>>> {
        self.handler.call_js(ctx, req, JsRouteContext {}).await
    }
}

#[derive(Clone)]
pub struct ResolvedRouter<'js> {
    tree: Rc<routing::router::Router<JsHandler<'js>>>,
    modifiers: ModifierList<reggie::Body, JsRouteContext>,
    debug: bool,
}

impl<'js> Trace<'js> for ResolvedRouter<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        for (_, i) in self.tree.iter() {
            for entry in &i.entries {
                entry.handler.trace(tracer);
            }
        }
    }
}

impl<'js> ResolvedRouter<'js> {
    pub fn match_route<P: Params>(
        &self,
        path: &str,
        method: MethodFilter,
        params: &mut P,
    ) -> Option<&JsHandler<'js>> {
        self.tree.match_route(path, method, params)
    }

    pub async fn handle(
        &self,
        ctx: Ctx<'_>,
        mut req: Request<reggie::Body>,
        context: JsRouteContext,
    ) -> rquickjs::Result<Response<reggie::Body>> {
        let mut params: HashMap<String, String> = HashMap::default();
        let Some(route) = self
            .match_route(req.uri().path(), req.method().clone().into(), &mut params)
            .cloned()
        else {
            let mut resp = Response::new(reggie::Body::empty());
            *resp.status_mut() = reggie::http::StatusCode::NOT_FOUND;
            return Ok(resp);
        };

        let modify = self.modifiers.before(&mut req, &context).await;

        let resp = route
            .call(
                req.map_body(|body| {
                    reggie::Body::from_streaming(body.map_err(reggie::Error::conn))
                }),
                JsRouteContext {},
            )
            .await;

        match resp {
            Ok(mut ret) => {
                modify.modify(&mut ret, &context).await;

                Ok(ret)
            }
            Err(err) => {
                let body = if self.debug {
                    reggie::Body::from(err.to_string())
                } else {
                    reggie::Body::empty()
                };

                let mut resp = Response::new(body);
                *resp.status_mut() = reggie::http::StatusCode::INTERNAL_SERVER_ERROR;
                Ok(resp)
            }
        }
    }
}

pub fn compose<'js>(middlewares: &[JsMiddleware<'js>], task: JsHandler<'js>) -> JsHandler<'js> {
    let mut iter = middlewares.iter();
    let Some(middleware) = iter.next() else {
        return task;
    };

    let mut handler = middleware.wrap(task);
    while let Some(middleware) = iter.next() {
        handler = middleware.wrap(handler);
    }

    handler
}
