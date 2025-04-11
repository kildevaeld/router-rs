use std::mem;

use klaver::{Options, Vm};
use rquickjs::Class;
use wilbur_quick::{App, InitList, InitModule, Router};

struct Move<'js> {
    router: Router<'js>,
}

unsafe impl<'js> Send for Move<'js> {}

unsafe impl<'js> Sync for Move<'js> {}

fn main() {}

async fn wrap() {
    let mut builder = InitList::default();

    builder.add_init(|app: &mut App<'_>| {
        app.add_module(wilbur_cookies::CookiesModule);
    });

    builder.add_init(InitModule(wilbur_cookies::CookiesModule));

    let vm = Options::default().build().await.unwrap();

    klaver::async_with!(vm => |ctx| {

        builder.build(ctx).await;

        Ok(())
    })
    .await;
}
