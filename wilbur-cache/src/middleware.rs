use core::mem;
use headers::{CacheControl, HeaderMapExt};
use heather::{HBoxFuture, HSend, HSendSync};
use keyval::Cbor;
use std::time::{Duration, Instant};
use wilbur_core::{
    Bytes, Error, Handler, HeaderMap, IntoResponse, Middleware, Request, Response, StatusCode,
    body::{Body as HttpBody, to_bytes},
};

pub type Store = keyval::KeyVal<Box<dyn keyval::TtlStore>>;

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedResponse {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Bytes,
}

impl CachedResponse {
    pub async fn from_response<B>(resp: &mut Response<B>) -> Result<CachedResponse, Error>
    where
        B: HttpBody + From<Bytes>,
        B::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        let body = mem::replace(resp.body_mut(), B::empty());
        let body = to_bytes(body).await?;

        *resp.body_mut() = B::from(body.clone());

        Ok(CachedResponse {
            status: resp.status(),
            headers: resp.headers().clone(),
            body,
        })
    }

    pub fn into_response<B>(self) -> Response<B>
    where
        B: HttpBody + From<Bytes>,
    {
        let mut resp = Response::new(self.body.into());

        *resp.headers_mut() = self.headers;
        *resp.status_mut() = self.status;

        resp
    }
}

// https://developers.cloudflare.com/cache/how-to/cache-keys/
fn create_cache_key<B>(req: &Request<B>) -> Bytes {
    let key = req.uri().to_string().into_bytes();
    key.into()
}

pub struct CacheMiddlware {
    store: Store,
    cache_time: Duration,
}

impl CacheMiddlware {
    pub fn new(store: Store, cache_time: Duration) -> CacheMiddlware {
        CacheMiddlware { store, cache_time }
    }
}

impl<B, C, H> Middleware<B, C, H> for CacheMiddlware
where
    H: Handler<B, C> + Clone + 'static,
    H::Response: IntoResponse<B>,
    B: From<Bytes> + HttpBody + 'static + HSend,
    B::Data: HSend,
    B::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    C: HSendSync + 'static,
{
    type Handle = CacheMiddlwareHandler<H>;

    fn wrap(&self, handler: H) -> Self::Handle {
        CacheMiddlwareHandler {
            handler,
            store: self.store.clone(),
            cache_time: self.cache_time,
        }
    }
}

pub struct CacheMiddlwareHandler<H> {
    handler: H,
    store: Store,
    cache_time: Duration,
}

impl<B, C, H> Handler<B, C> for CacheMiddlwareHandler<H>
where
    H: Handler<B, C> + Clone + 'static,
    H::Response: IntoResponse<B>,
    B: From<Bytes> + HttpBody + 'static + HSend,
    B::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    B::Data: HSend,
    C: HSendSync + 'static,
{
    type Response = Response<B>;

    type Future<'a>
        = HBoxFuture<'a, Result<Self::Response, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a> {
        Box::pin(async move {
            let ret = match run(req, context, &self.store, self.cache_time, &self.handler).await {
                Ok(ret) => ret,
                Err(err) => {
                    let mut resp = Response::new(Bytes::from(err.to_string()).into());
                    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    resp
                }
            };

            Ok(ret)
        })
    }
}

async fn run<B, C, T>(
    req: Request<B>,
    context: &C,
    store: &Store,
    timeout: Duration,
    handler: &T,
) -> Result<Response<B>, Error>
where
    T: Handler<B, C> + Clone + 'static,
    T::Response: IntoResponse<B>,
    B: From<Bytes> + HttpBody,
    B::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    // Generate a unique cache key based on the request URI
    let cache_key = create_cache_key(&req);

    // Check if the request has a `Cache-Control` header with `no-cache` directive
    let no_cache = if let Some(header) = req.headers().typed_get::<CacheControl>() {
        header.no_cache()
    } else {
        false
    };

    // If caching is allowed (no `no-cache` directive)
    if !no_cache {
        // Attempt to retrieve a cached response from the store
        if let Ok(Cbor(found)) = store.get::<_, Cbor<CachedResponse>>(&cache_key).await {
            // If a cached response is found, return it as the response
            return Ok(found.into_response());
        }
    }

    // Call the handler to process the request and generate a response
    let mut response = handler.call(context, req).await?.into_response();

    // Determine the cache timeout based on the response's `Cache-Control` header
    let timeout = if let Some(header) = response.headers_mut().typed_get::<CacheControl>() {
        if header.no_cache() {
            // If the response has a `no-cache` directive, return it without caching
            return Ok(response);
        }

        // Use `max-age` or `s-maxage` if available, otherwise fallback to the default timeout
        header
            .max_age()
            .or_else(|| header.s_max_age())
            .unwrap_or(timeout)
    } else {
        // If no `Cache-Control` header is present, use the default timeout
        timeout
    };

    // Add a `Cache-Control` header to the response with the determined timeout
    response
        .headers_mut()
        .typed_insert(CacheControl::new().with_max_age(timeout));

    // Cache the response in the store with a TTL (time-to-live) based on the timeout
    store
        .insert_ttl(
            cache_key,
            Cbor(CachedResponse::from_response(&mut response).await?),
            Instant::now().checked_add(timeout).unwrap(), // Calculate the expiration time
        )
        .await
        .map_err(|err| Error::new(err))?; // Handle any errors during the caching process

    Ok(response)
}
