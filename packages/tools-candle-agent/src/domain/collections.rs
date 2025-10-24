//! Candle Collections - Standalone collection types

use serde::{Deserialize, Serialize};

/// Candle-specific collection type supporting zero, one, or many items
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ZeroOneOrMany<T> {
    /// No items
    #[default]
    None,
    /// Exactly one item
    One(T),
    /// Multiple items
    Many(Vec<T>),
}

impl<T> ZeroOneOrMany<T> {
    /// Create a new collection with one item
    pub fn one(item: T) -> Self {
        Self::One(item)
    }

    /// Create a new collection with many items
    #[must_use]
    pub fn many(items: Vec<T>) -> Self {
        if items.is_empty() {
            Self::None
        } else if items.len() == 1 {
            // Safe: we verified len() == 1 above
            match items.into_iter().next() {
                Some(item) => Self::One(item),
                None => Self::None, // Fallback in impossible case
            }
        } else {
            Self::Many(items)
        }
    }

    /// Push an item, returning a new collection
    #[must_use]
    pub fn with_pushed(self, item: T) -> Self {
        match self {
            Self::None => Self::One(item),
            Self::One(existing) => Self::Many(vec![existing, item]),
            Self::Many(mut items) => {
                items.push(item);
                Self::Many(items)
            }
        }
    }

    /// Convert to Vec, always returning a Vec regardless of variant
    pub fn to_vec(self) -> Vec<T> {
        match self {
            Self::None => vec![],
            Self::One(item) => vec![item],
            Self::Many(items) => items,
        }
    }

    /// Get the length/count of items
    pub fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::One(_) => 1,
            Self::Many(items) => items.len(),
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Iterate over the items
    pub fn iter(&self) -> ZeroOneOrManyIter<'_, T> {
        match self {
            Self::None => ZeroOneOrManyIter::None,
            Self::One(item) => ZeroOneOrManyIter::One(Some(item)),
            Self::Many(items) => ZeroOneOrManyIter::Many(items.iter()),
        }
    }
}

impl<T> IntoIterator for ZeroOneOrMany<T> {
    type Item = T;
    type IntoIter = ZeroOneOrManyIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::None => ZeroOneOrManyIntoIter::None,
            Self::One(item) => ZeroOneOrManyIntoIter::One(Some(item)),
            Self::Many(items) => ZeroOneOrManyIntoIter::Many(items.into_iter()),
        }
    }
}

impl<'a, T> IntoIterator for &'a ZeroOneOrMany<T> {
    type Item = &'a T;
    type IntoIter = ZeroOneOrManyIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator for `ZeroOneOrMany` references
pub enum ZeroOneOrManyIter<'a, T> {
    /// No items to iterate
    None,
    /// Single item to iterate
    One(Option<&'a T>),
    /// Multiple items to iterate
    Many(std::slice::Iter<'a, T>),
}

impl<'a, T> Iterator for ZeroOneOrManyIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::None => None,
            Self::One(item) => item.take(),
            Self::Many(iter) => iter.next(),
        }
    }
}

/// `IntoIterator` for `ZeroOneOrMany`
pub enum ZeroOneOrManyIntoIter<T> {
    /// No items to iterate
    None,
    /// Single item to iterate
    One(Option<T>),
    /// Multiple items to iterate
    Many(std::vec::IntoIter<T>),
}

impl<T> Iterator for ZeroOneOrManyIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::None => None,
            Self::One(item) => item.take(),
            Self::Many(iter) => iter.next(),
        }
    }
}

impl<T> From<T> for ZeroOneOrMany<T> {
    fn from(item: T) -> Self {
        Self::One(item)
    }
}

impl<T> From<Vec<T>> for ZeroOneOrMany<T> {
    fn from(items: Vec<T>) -> Self {
        Self::many(items)
    }
}

impl<T> From<Option<T>> for ZeroOneOrMany<T> {
    fn from(option: Option<T>) -> Self {
        match option {
            Some(item) => Self::One(item),
            None => Self::None,
        }
    }
}
