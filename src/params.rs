#[cfg(not(feature = "std"))]
use alloc::{borrow::Cow, collections::BTreeMap, vec::Vec};
#[cfg(feature = "std")]
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

pub trait Params<'a> {
    fn set(&mut self, key: Cow<'a, str>, value: &'a str);
}

impl<'a> Params<'a> for BTreeMap<Cow<'a, str>, &'a str> {
    fn set(&mut self, key: Cow<'a, str>, value: &'a str) {
        self.insert(key, value);
    }
}

#[cfg(feature = "std")]
impl<'a> Params<'a> for HashMap<Cow<'a, str>, &'a str> {
    fn set(&mut self, key: Cow<'a, str>, value: &'a str) {
        self.insert(key, value);
    }
}
