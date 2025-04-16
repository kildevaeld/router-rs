use std::sync::Arc;

use rquickjs_modules::Environ;

use crate::Init;

pub struct App {
    inits: Vec<Arc<dyn Init + Send + Sync>>,
}

impl App {
    pub async fn build(self) {}
}
