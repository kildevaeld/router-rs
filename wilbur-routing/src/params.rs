use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct UrlParams {
    pub(crate) inner: HashMap<Arc<str>, Arc<str>>,
}

impl UrlParams {
    pub fn get(&self, name: &str) -> Option<&Arc<str>> {
        self.inner.get(name)
    }
}
