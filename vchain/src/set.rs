use crate::digest::Digestible;
use core::iter::FromIterator;
use core::ops::{Add, BitAnd, BitOr, Deref};
use serde::{
    de::Deserializer,
    ser::{SerializeSeq, SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use std::collections::HashMap;

pub trait SetElement: Digestible + Clone + Send + Sync + Eq + PartialEq + core::hash::Hash {}

impl<T> SetElement for T where
    T: Digestible + Clone + Send + Sync + Eq + PartialEq + core::hash::Hash
{
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MultiSet<T: SetElement> {
    pub(crate) inner: HashMap<T, u32>,
}

impl<T: SetElement> MultiSet<T> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn from_vec(input: Vec<T>) -> Self {
        Self::from_iter(input.into_iter())
    }

    pub fn from_tuple_vec(input: Vec<(T, u32)>) -> Self {
        Self::from_iter(input.into_iter())
    }

    pub fn is_intersected_with(&self, other: &Self) -> bool {
        let (a, b) = if self.len() < other.len() {
            (self, other)
        } else {
            (other, self)
        };
        a.keys().any(|v| b.contains_key(v))
    }
}

impl<T: SetElement> Deref for MultiSet<T> {
    type Target = HashMap<T, u32>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'b, T: SetElement> Add<&'a MultiSet<T>> for &'b MultiSet<T> {
    type Output = MultiSet<T>;

    fn add(self, other: &'a MultiSet<T>) -> MultiSet<T> {
        let mut data = HashMap::new();
        for (k, v) in self.iter().chain(other.iter()) {
            *data.entry(k.clone()).or_insert(0) += v;
        }
        MultiSet { inner: data }
    }
}

impl<'a, 'b, T: SetElement> BitOr<&'a MultiSet<T>> for &'b MultiSet<T> {
    type Output = MultiSet<T>;

    fn bitor(self, other: &'a MultiSet<T>) -> MultiSet<T> {
        let mut data = HashMap::new();
        for k in self.keys().chain(other.keys()) {
            data.entry(k.clone()).or_insert(1);
        }
        MultiSet { inner: data }
    }
}

impl<'a, 'b, T: SetElement> BitAnd<&'a MultiSet<T>> for &'b MultiSet<T> {
    type Output = MultiSet<T>;

    fn bitand(self, other: &'a MultiSet<T>) -> MultiSet<T> {
        let mut data = HashMap::new();
        for k in self.keys() {
            if other.contains_key(k) {
                data.insert(k.clone(), 1);
            }
        }
        MultiSet { inner: data }
    }
}

impl<T: SetElement> FromIterator<T> for MultiSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut data = HashMap::new();
        for d in iter {
            *data.entry(d).or_insert(0) += 1;
        }
        Self { inner: data }
    }
}

impl<T: SetElement> FromIterator<(T, u32)> for MultiSet<T> {
    fn from_iter<I: IntoIterator<Item = (T, u32)>>(iter: I) -> Self {
        let mut data = HashMap::new();
        for (k, v) in iter {
            *data.entry(k).or_insert(0) += v;
        }
        Self { inner: data }
    }
}

#[derive(Serialize, Deserialize)]
struct ElementTuple<T> {
    obj: T,
    cnt: u32,
}

impl<T: SetElement + Serialize> Serialize for MultiSet<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let mut seq = serializer.serialize_seq(Some(self.len()))?;
            for (k, v) in self.iter() {
                seq.serialize_element(&ElementTuple {
                    obj: k.clone(),
                    cnt: *v,
                })?;
            }
            seq.end()
        } else {
            let mut state = serializer.serialize_struct("MultiSet", 1)?;
            state.serialize_field("inner", &self.inner)?;
            state.end()
        }
    }
}

impl<'de, T: SetElement + Deserialize<'de>> Deserialize<'de> for MultiSet<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let inner: Vec<ElementTuple<T>> = Deserialize::deserialize(deserializer)?;
            Ok(Self::from_iter(inner.into_iter().map(|v| (v.obj, v.cnt))))
        } else {
            let inner: HashMap<T, u32> = Deserialize::deserialize(deserializer)?;
            Ok(Self { inner })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_intersected_with() {
        let s1 = MultiSet::from_vec(vec![1, 2, 3]);
        let s2 = MultiSet::from_vec(vec![2, 2, 5]);
        let s3 = MultiSet::from_vec(vec![5, 6]);
        assert!(s1.is_intersected_with(&s2));
        assert!(!s1.is_intersected_with(&s3));
    }

    #[test]
    fn test_set_sum() {
        let s1 = MultiSet::from_vec(vec![1, 1, 2]);
        let s2 = MultiSet::from_vec(vec![2, 2, 3]);
        let s3 = MultiSet::from_tuple_vec(vec![(1, 2), (2, 3), (3, 1)]);
        assert_eq!(&s1 + &s2, s3);
    }

    #[test]
    fn test_set_union() {
        let s1 = MultiSet::from_vec(vec![1, 1, 2]);
        let s2 = MultiSet::from_vec(vec![2, 2, 3]);
        let s3 = MultiSet::from_tuple_vec(vec![(1, 1), (2, 1), (3, 1)]);
        assert_eq!(&s1 | &s2, s3);
    }

    #[test]
    fn test_set_intersection() {
        let s1 = MultiSet::from_vec(vec![1, 1, 2]);
        let s2 = MultiSet::from_vec(vec![2, 2, 3]);
        let s3 = MultiSet::from_tuple_vec(vec![(2, 1)]);
        assert_eq!(&s1 & &s2, s3);
    }

    #[test]
    fn test_serde() {
        let s = MultiSet::from_vec(vec![1, 1, 2]);
        let json = serde_json::to_string_pretty(&s).unwrap();
        let bin = bincode::serialize(&s).unwrap();
        assert_eq!(serde_json::from_str::<MultiSet<i32>>(&json).unwrap(), s);
        assert_eq!(bincode::deserialize::<MultiSet<i32>>(&bin[..]).unwrap(), s);
    }
}
