use keyval::{Cbor, StoreExt, TtlStore};
use uuid::Uuid;
use vaerdi::Map;

#[derive(Clone)]
pub struct SessionStore {
    store: keyval::KeyVal<Box<dyn keyval::TtlStore>>,
}

impl Default for SessionStore {
    fn default() -> Self {
        let store =
            keyval::KeyVal::new(Box::new(keyval::Memory::new().into_ttl()) as Box<dyn TtlStore>);
        SessionStore { store }
    }
}

impl SessionStore {
    pub async fn load(&self, id: &Uuid) -> Option<Map> {
        self.store
            .get::<_, Cbor<Map>>(&id.as_bytes()[..])
            .await
            .ok()
            .map(|m| m.0)
    }

    pub async fn save(&self, id: &Uuid, value: &Map) {
        self.store
            .insert(&id.as_bytes()[..], Cbor(value.clone()))
            .await
            .ok();
    }

    pub async fn remove(&self, id: &Uuid) {
        self.store.remove(&&id.as_bytes()[..]).await.ok();
    }
}
