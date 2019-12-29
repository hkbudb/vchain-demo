use blake2b_simd::{self, blake2bp};
use serde::{
    de::{Deserializer, SeqAccess, Visitor},
    ser::{SerializeTupleStruct, Serializer},
    Deserialize, Serialize,
};

pub const DIGEST_LEN: usize = 32;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub struct Digest(pub [u8; DIGEST_LEN]);

// Ref: https://github.com/slowli/hex-buffer-serde

impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(&self.0))
        } else {
            let mut state = serializer.serialize_tuple_struct("Digest", 1)?;
            state.serialize_field(&self.0)?;
            state.end()
        }
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as DeError;
        use std::fmt;

        struct HexVisitor;

        impl<'de> Visitor<'de> for HexVisitor {
            type Value = Digest;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct Digest")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<Digest, E> {
                let data = hex::decode(value).map_err(E::custom)?;
                if data.len() == DIGEST_LEN {
                    let mut out = Digest::default();
                    out.0.copy_from_slice(&data[..DIGEST_LEN]);
                    Ok(out)
                } else {
                    Err(E::custom(format!("invalid length: {}", data.len())))
                }
            }
        }

        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Digest;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct Digest")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Digest, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let inner = seq
                    .next_element()?
                    .ok_or_else(|| DeError::invalid_length(0, &self))?;
                Ok(Digest(inner))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(HexVisitor)
        } else {
            deserializer.deserialize_tuple_struct("Digest", 1, BytesVisitor)
        }
    }
}

impl From<blake2b_simd::Hash> for Digest {
    fn from(input: blake2b_simd::Hash) -> Self {
        let data = input.as_bytes();
        debug_assert_eq!(data.len(), DIGEST_LEN);
        let mut out = Self::default();
        out.0.copy_from_slice(&data[..DIGEST_LEN]);
        out
    }
}

pub fn blake2() -> blake2bp::Params {
    let mut params = blake2bp::Params::new();
    params.hash_length(DIGEST_LEN);
    params
}

pub trait Digestable {
    fn to_digest(&self) -> Digest;
}

impl Digestable for [u8] {
    fn to_digest(&self) -> Digest {
        Digest::from(blake2().hash(self))
    }
}

impl Digestable for str {
    fn to_digest(&self) -> Digest {
        self.as_bytes().to_digest()
    }
}

impl Digestable for String {
    fn to_digest(&self) -> Digest {
        self.as_bytes().to_digest()
    }
}

macro_rules! impl_digestable_for_numeric {
    ($x: ty) => {
        impl Digestable for $x {
            fn to_digest(&self) -> Digest {
                self.to_le_bytes().to_digest()
            }
        }
    };
    ($($x: ty),*) => {$(impl_digestable_for_numeric!($x);)*}
}

impl_digestable_for_numeric!(i8, i16, i32, i64, i128);
impl_digestable_for_numeric!(u8, u16, u32, u64, u128);
impl_digestable_for_numeric!(f32, f64);

pub fn concat_digest_ref<'a>(input: impl Iterator<Item = &'a Digest>) -> Digest {
    let mut state = blake2().to_state();
    for d in input {
        state.update(&d.0);
    }
    Digest::from(state.finalize())
}

pub fn concat_digest(input: impl Iterator<Item = Digest>) -> Digest {
    let mut state = blake2().to_state();
    for d in input {
        state.update(&d.0);
    }
    Digest::from(state.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_digest() {
        let expect = Digest(*b"\xbd\x86\xc3\x39\x7e\x8f\x3a\x9f\xc6\x95\xd1\xba\x57\x40\x86\xa1\x34\x55\x4c\xea\x08\xec\x9c\x9e\x65\xdd\xbb\x5b\x82\x3e\x8c\x03");
        assert_eq!(b"hello"[..].to_digest(), expect);
        assert_eq!("hello".to_digest(), expect);
        assert_eq!("hello".to_owned().to_digest(), expect);
    }

    #[test]
    fn test_digest_concat() {
        let input = vec!["hello".to_digest(), "world!".to_digest()];
        let expect = {
            let mut buf: Vec<u8> = Vec::new();
            buf.extend_from_slice(&input[0].0[..]);
            buf.extend_from_slice(&input[1].0[..]);
            buf.as_slice().to_digest()
        };
        assert_eq!(concat_digest_ref(input.iter()), expect);
        assert_eq!(concat_digest(input.into_iter()), expect);
    }

    #[test]
    fn test_serde() {
        let digest = "hello".to_digest();
        let json = serde_json::to_string_pretty(&digest).unwrap();
        assert_eq!(
            json,
            "\"bd86c3397e8f3a9fc695d1ba574086a134554cea08ec9c9e65ddbb5b823e8c03\""
        );
        let bin = bincode::serialize(&digest).unwrap();
        assert_eq!(
            bin,
            b"\xbd\x86\xc3\x39\x7e\x8f\x3a\x9f\xc6\x95\xd1\xba\x57\x40\x86\xa1\x34\x55\x4c\xea\x08\xec\x9c\x9e\x65\xdd\xbb\x5b\x82\x3e\x8c\x03",
        );

        assert_eq!(serde_json::from_str::<Digest>(&json).unwrap(), digest);
        assert_eq!(bincode::deserialize::<Digest>(&bin[..]).unwrap(), digest);
    }
}
