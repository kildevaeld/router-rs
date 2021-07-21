use criterion::{black_box, criterion_group, criterion_main, Criterion};
use router::{Route, Router, Segment};

use std::{collections::HashMap, vec::Vec};

fn fibonacci<'a>(graph: &'a Router<String>, path: &str) -> Option<&'a Vec<String>> {
    graph.find(path, &mut HashMap::default())
}

fn fibonacci2<'a>(routes: &Vec<Route<'static>>, path: &str) -> Option<usize> {
    for (kv, route) in routes.iter().enumerate() {
        if route.match_path(path, &mut HashMap::default()) {
            return Some(kv);
        }
    }

    None
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut graph = Router::new();
    let mut routes = Vec::new();
    for i in 0..10000 {
        graph.register(
            &[Segment::Constant(format!("test{}", i).into())],
            String::new(),
        );
        let s = format!("/test{}", i);
        routes.push(Route::new(&s).unwrap().to_static());
    }
    graph.register(&[Segment::Constant("hello".into())], String::new());
    routes.push(Route::new("/hello").unwrap());

    c.bench_function("graph", |b| {
        //
        b.iter(|| fibonacci(&graph, black_box("hello")))
    });
    c.bench_function("routes", |b| {
        //
        b.iter(|| fibonacci2(&routes, black_box("hello")))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
