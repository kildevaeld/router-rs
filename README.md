## Router

```rust

let mut router = Router::new();

router.register("/", "index")?
    .register("/news", "News List")?
    .register("/news/:id", "News Item")?;

let mut params = HashMap::default();
if let Some(route) = router.find("/news/100", &mut params) {
    println!("{}: {}", route, params.get("id").unwrap()); // prints: News Item: 100
}

```
