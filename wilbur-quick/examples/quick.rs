use klaver::{Options, RuntimeError};
use wilbur_quick::{App, AppBuilder, InitModule, InitPath, Router};

fn main() {
    futures::executor::block_on(async move {
        wrap().await.unwrap();
    })
}

async fn wrap() -> Result<(), RuntimeError> {
    let mut builder = AppBuilder::default();

    builder.add_init(|app: &mut App<'_>| {
        app.add_module(wilbur_cookies::CookiesModule);
    });

    builder.add_init(InitModule(wilbur_cookies::CookiesModule));

    builder.add_init(InitPath("./wilbur-quick/examples/app.js"));

    let vm = Options::default().search_path(".").build().await.unwrap();

    klaver::async_with!(vm => |ctx| {

        builder.build(ctx).await?;

        Ok(())
    })
    .await?;

    Ok(())
}
