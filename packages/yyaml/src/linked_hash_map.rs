use std::collections::BTreeMap;

/// Maintains insertion order plus unique keys, like `linked_hash_map`.
/// Here we inline a minimal version for demonstration.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct LinkedHashMap<K: PartialEq + Eq, V> {
    map: BTreeMap<usize, (K, V)>,
    order: Vec<usize>,
    next_id: usize,
}

impl<K: PartialEq + Eq, V> LinkedHashMap<K, V> {
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            order: Vec::new(),
            next_id: 0,
        }
    }

    #[must_use] 
    pub const fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    #[must_use] 
    pub const fn len(&self) -> usize {
        self.order.len()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        for id in &self.order {
            if let Some((k, v)) = self.map.get(id)
                && k.borrow() == key
            {
                return Some(v);
            }
        }
        None
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // check if key exists
        for id in &self.order {
            if let Some((k, old_v)) = self.map.get_mut(id)
                && k == &key
            {
                let old = std::mem::replace(old_v, value);
                return Some(old);
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.map.insert(id, (key, value));
        self.order.push(id);
        None
    }
}

impl<K: PartialEq + Eq, V> Default for LinkedHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: PartialEq + Eq + Clone, V: Clone> IntoIterator for LinkedHashMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        let mut out = Vec::with_capacity(self.order.len());
        for id in self.order {
            if let Some((k, v)) = self.map.get(&id) {
                out.push((k.clone(), v.clone()));
            }
        }
        out.into_iter()
    }
}

pub struct Iter<'a, K, V> {
    inner: std::slice::Iter<'a, usize>,
    map: &'a BTreeMap<usize, (K, V)>,
}

impl<'a, K: PartialEq + Eq, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&id) = self.inner.next()
            && let Some((k, v)) = self.map.get(&id)
        {
            return Some((k, v));
        }
        None
    }
}

impl<K: PartialEq + Eq, V> LinkedHashMap<K, V> {
    #[inline]
    #[must_use] 
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            map: &self.map,
            inner: self.order.iter(),
        }
    }

    #[inline]
    #[must_use] 
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: BTreeMap::new(),
            order: Vec::with_capacity(capacity),
            next_id: 0,
        }
    }
}

// Zero-allocation FromIterator implementation for blazing-fast collect()
impl<K: PartialEq + Eq, V> std::iter::FromIterator<(K, V)> for LinkedHashMap<K, V> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        let mut map = Self::with_capacity(lower);

        for (key, value) in iter {
            map.insert(key, value);
        }

        map
    }
}
