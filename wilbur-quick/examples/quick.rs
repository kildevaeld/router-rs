use std::net::SocketAddr;

use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
// use klaver::{Options, RuntimeError, pool::VmPoolOptions};
use reggie::Body;
use rquickjs::{Class, Ctx};
use tokio::net::TcpListener;
use wilbur_quick::{AppBuildCtx, AppBuilder, InitModule, InitPath, JsApp, Router};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    let mut builder = AppBuilder::default();

    builder.add_init(|app: &mut AppBuildCtx<'_>| {
        app.add_module(wilbur_cookies::CookiesModule);
    });

    builder.add_init(InitModule(wilbur_cookies::CookiesModule));

    builder.add_init(InitPath("./wilbur-quick/examples/app.js"));

    builder
        .build()
        .serve(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .await?;

    Ok(())
}
