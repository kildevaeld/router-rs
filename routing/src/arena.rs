use std::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Id(usize);

#[derive(Debug)]
pub struct Arena<T> {
    inner: Vec<T>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Arena {
            inner: Default::default(),
        }
    }
}

impl<T> Clone for Arena<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Arena {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Arena<T> {
    pub fn alloc(&mut self, item: T) -> Id {
        let index = self.inner.len();
        self.inner.push(item);
        Id(index)
    }

    pub fn get(&self, id: Id) -> Option<&T> {
        self.inner.get(id.0)
    }

    pub fn map<F, V>(self, mapper: F) -> Arena<V>
    where
        F: FnMut(T) -> V,
    {
        Arena {
            inner: self.inner.into_iter().map(mapper).collect(),
        }
    }
}

impl<T> core::ops::Index<Id> for Arena<T> {
    type Output = T;

    fn index(&self, index: Id) -> &Self::Output {
        self.inner.get(index.0).unwrap()
    }
}

impl<T> core::ops::IndexMut<Id> for Arena<T> {
    fn index_mut(&mut self, index: Id) -> &mut Self::Output {
        self.inner.get_mut(index.0).unwrap()
    }
}
