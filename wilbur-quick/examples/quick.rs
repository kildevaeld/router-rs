use klaver::{Options, RuntimeError};
use wilbur_quick::{App, AppBuilder, InitModule, Router};

struct Move<'js> {
    router: Router<'js>,
}

unsafe impl<'js> Send for Move<'js> {}

unsafe impl<'js> Sync for Move<'js> {}

fn main() {
    futures::executor::block_on(async move {
        wrap().await;
    })
}

async fn wrap() -> Result<(), RuntimeError> {
    let mut builder = AppBuilder::default();

    builder.add_init(|app: &mut App<'_>| {
        app.add_module(wilbur_cookies::CookiesModule);
    });

    builder.add_init(InitModule(wilbur_cookies::CookiesModule));

    let vm = Options::default().build().await.unwrap();

    klaver::async_with!(vm => |ctx| {

        builder.build(ctx).await;

        Ok(())
    })
    .await?;

    Ok(())
}
