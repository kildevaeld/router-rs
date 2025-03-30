use criterion::{black_box, criterion_group, criterion_main, Criterion};
use routing::{match_path, parse, Params, ParseError, Router, Segment, Segments};
use std::{collections::BTreeMap, vec::Vec};

#[derive(Clone, Debug, PartialEq)]
pub struct Route<'a> {
    pub(crate) segments: Segments<'a>,
}

impl<'a> Route<'a> {
    pub fn new(path: &'a str) -> Result<Route<'a>, ParseError> {
        Ok(Route {
            segments: parse(path)?,
        })
    }

    pub fn match_path<'b, P: Params<'b>>(&self, path: &'b str, params: &'b mut P) -> bool
    where
        'a: 'b,
    {
        match_path(&self.segments, path, params)
    }

    pub fn to_static(self) -> Route<'static> {
        Route {
            segments: self.segments.to_owned(),
        }
    }
}

fn find<'a>(graph: &'a Router<String>, path: &str) -> Option<&'a Vec<String>> {
    graph.find(path, &mut BTreeMap::default())
}

fn find2<'a>(routes: &Vec<Route<'static>>, path: &str) -> Option<usize> {
    for (kv, route) in routes.iter().enumerate() {
        if route.match_path(path, &mut BTreeMap::default()) {
            return Some(kv);
        }
    }

    None
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut graph = Router::new();
    let mut routes = Vec::new();
    for i in 0..10000 {
        graph
            .register(
                &[Segment::Constant(format!("test{}", i).into())],
                String::new(),
            )
            .expect("router");
        let s = format!("/test{}", i);
        routes.push(Route::new(&s).unwrap().to_static());
    }
    graph
        .register(&[Segment::Constant("hello".into())], String::new())
        .expect("register");
    routes.push(Route::new("/hello").unwrap());

    c.bench_function("last graph", |b| {
        //
        b.iter(|| find(&graph, black_box("hello")))
    });
    c.bench_function("last router", |b| {
        //
        b.iter(|| find2(&routes, black_box("hello")))
    });

    c.bench_function("first graph", |b| {
        //
        b.iter(|| find(&graph, black_box("test1")))
    });
    c.bench_function("first router", |b| {
        //
        b.iter(|| find2(&routes, black_box("test1")))
    });

    c.bench_function("fifth graph", |b| {
        //
        b.iter(|| find(&graph, black_box("test5")))
    });
    c.bench_function("fifth router", |b| {
        //
        b.iter(|| find2(&routes, black_box("test5")))
    });

    c.bench_function("tenth graph", |b| {
        //
        b.iter(|| find(&graph, black_box("test10")))
    });
    c.bench_function("tenth router", |b| {
        //
        b.iter(|| find2(&routes, black_box("test10")))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
