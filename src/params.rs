use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

pub trait Params<'a> {
    fn set(&mut self, key: Cow<'a, str>, value: Cow<'a, str>);
}

impl<'a> Params<'a> for BTreeMap<Cow<'a, str>, Cow<'a, str>> {
    fn set(&mut self, key: Cow<'a, str>, value: Cow<'a, str>) {
        self.insert(key, value);
    }
}

impl<'a> Params<'a> for HashMap<Cow<'a, str>, Cow<'a, str>> {
    fn set(&mut self, key: Cow<'a, str>, value: Cow<'a, str>) {
        self.insert(key, value);
    }
}

impl<'a> Params<'a> for () {
    fn set(&mut self, _key: Cow<'a, str>, _value: Cow<'a, str>) {}
}
