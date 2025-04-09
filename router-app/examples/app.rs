use std::net::SocketAddr;

use http::{Request, Response};
use router::{MethodFilter, Routing, handle_fn, handler};
use router_app::{App, Body};
use router_session::{Session, SessionModule};
use tokio::task::LocalSet;

use router_cookies::{CookieJar, CookiesModule};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let local_set = LocalSet::new();

    local_set
        .run_until(async move {
            //

            let mut app = App::new();

            app.add_module(CookiesModule);
            app.add_module(SessionModule::default());

            app.route(
                MethodFilter::GET,
                "/",
                handle_fn(|state, req| async move {
                    //
                    Result::<_, router::Error>::Ok(Response::new(Body::from("Hello, World!")))
                }),
            )
            .unwrap();

            app.route(
                MethodFilter::GET,
                "/sub",
                handler(|mut session: Session, cookies: CookieJar| async move {
                    session.set("name", "Hello, World!".into());
                    session.save().await;
                    "Hello, World"
                }),
            )
            .unwrap();

            app.serve(SocketAddr::from(([127, 0, 0, 1], 3000))).await?;

            Result::<_, router_app::Error>::Ok(())
        })
        .await
        .unwrap();
}
