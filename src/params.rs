use alloc::{borrow::Cow, collections::btree_map::BTreeMap};

pub trait Params<'a> {
    fn set(&mut self, key: Cow<'a, str>, value: Cow<'a, str>);
}

impl<'a> Params<'a> for BTreeMap<Cow<'a, str>, Cow<'a, str>> {
    fn set(&mut self, key: Cow<'a, str>, value: Cow<'a, str>) {
        self.insert(key, value);
    }
}

#[cfg(feature = "std")]
impl<'a> Params<'a> for std::collections::HashMap<Cow<'a, str>, Cow<'a, str>> {
    fn set(&mut self, key: Cow<'a, str>, value: Cow<'a, str>) {
        self.insert(key, value);
    }
}

impl<'a> Params<'a> for () {
    fn set(&mut self, _key: Cow<'a, str>, _value: Cow<'a, str>) {}
}
