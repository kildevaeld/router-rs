use arc_swap::{ArcSwap, ArcSwapAny};
use heather::{HBoxFuture, HSend, HSendSync};
use std::sync::Arc;
use uuid::Uuid;
use vaerdi::{Map, Value, hashbrown::hash_map::Iter};
use wilbur_container::{Extensible, ReadableContainer};
use wilbur_core::{Error, FromRequest, FromRequestParts};

use crate::session_store::SessionStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Set(Uuid),
    Remove(Uuid),
    Init(Uuid),
    Noop,
}

impl State {
    pub fn id(&self) -> Option<Uuid> {
        match self {
            Self::Remove(id) => Some(*id),
            Self::Set(id) => Some(*id),
            Self::Init(id) => Some(*id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionId(pub(crate) Arc<ArcSwap<State>>);

impl Default for SessionId {
    fn default() -> Self {
        SessionId(Arc::new(ArcSwapAny::new(State::Noop.into())))
    }
}

impl SessionId {
    pub fn new(id: Uuid) -> SessionId {
        SessionId(Arc::new(ArcSwapAny::new(State::Init(id).into())))
    }

    pub(crate) fn state(&self) -> State {
        **self.0.load()
    }

    fn remove(&self) {
        let state = self.state();
        if let Some(id) = state.id() {
            self.0.store(State::Remove(id).into());
        }
    }

    fn generate(&self) {
        self.0.store(State::Set(Uuid::new_v4()).into());
    }
}

pub struct Session {
    id: SessionId,
    store: SessionStore,
    value: Map,
}

impl Session {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.value.get(key)
    }

    pub fn set(&mut self, key: &str, value: Value) -> Option<Value> {
        self.value.insert(key, value)
    }

    pub async fn load(&mut self) {
        if let Some(id) = self.id.state().id() {
            if let Some(ret) = self.store.load(&id).await {
                self.value = ret;
            }
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.value.remove(name);
    }

    pub async fn regenerate_id(&mut self) {
        if let Some(id) = self.id.state().id() {
            self.store.remove(&id).await;
        }
        self.id.generate();
        self.save().await;
    }

    pub async fn save(&mut self) {
        if self.id.state().id().is_none() {
            self.id.generate();
        }
        if let Some(id) = self.id.state().id() {
            self.store.save(&id, &self.value).await;
        }
    }

    pub async fn delete(&mut self) {
        if let Some(id) = self.id.state().id() {
            self.store.remove(&id).await;
        }
        self.id.remove();
    }

    pub fn iter(&self) -> Iter<'_, vaerdi::String, Value> {
        self.value.iter()
    }
}

impl<C: Extensible + HSendSync> FromRequestParts<C> for Session {
    type Future<'a>
        = HBoxFuture<'a, Result<Session, Error>>
    where
        C: 'a;
    fn from_request_parts<'a>(
        parts: &'a mut http::request::Parts,
        state: &'a C,
    ) -> Self::Future<'a> {
        Box::pin(async move {
            let Some(store) = state.get::<SessionStore>() else {
                return Err(Error::new("session store not found"));
            };

            let Some(id) = parts.extensions.get::<SessionId>() else {
                return Err(Error::new("session not found"));
            };

            let map = if let Some(id) = id.state().id() {
                store.load(&id).await.unwrap_or_default()
            } else {
                Map::default()
            };

            Ok(Self {
                id: id.clone(),
                store: store.clone(),
                value: map,
            })
        })
    }
}

impl<B: 'static + HSend, C: Extensible + HSendSync> FromRequest<B, C> for Session {
    type Future<'a>
        = HBoxFuture<'a, Result<Session, Error>>
    where
        C: 'a;

    fn from_request<'a>(parts: http::Request<B>, state: &'a C) -> Self::Future<'a> {
        Box::pin(async move {
            let (mut parts, _) = parts.into_parts();
            Self::from_request_parts(&mut parts, state).await
        })
    }
}
