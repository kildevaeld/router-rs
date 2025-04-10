use std::net::SocketAddr;

use tokio::task::LocalSet;

use wilbur_app::{App, handler, prelude::*};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let local_set = LocalSet::new();

    local_set
        .run_until(async move {
            //

            let mut app = App::new();

            app.get("/", handler(|| async move { "Hello, World!" }))
                .unwrap();

            app.serve(SocketAddr::from(([127, 0, 0, 1], 3000)))
                .await
                .unwrap();

            Result::<_, wilbur_core::Error>::Ok(())
        })
        .await
        .unwrap();
}
