use core::fmt;
use serde::{
    de::{Deserializer, SeqAccess, Visitor},
    ser::{SerializeTupleStruct, Serializer},
    Deserialize, Serialize,
};

pub const DIGEST_LEN: usize = 32;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Default)]
pub struct Digest(pub [u8; DIGEST_LEN]);

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

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

pub fn blake2() -> blake2b_simd::Params {
    let mut params = blake2b_simd::Params::new();
    params.hash_length(DIGEST_LEN);
    params
}

pub trait Digestible {
    fn to_digest(&self) -> Digest;
}

impl Digestible for [u8] {
    fn to_digest(&self) -> Digest {
        Digest::from(blake2().hash(self))
    }
}

impl Digestible for str {
    fn to_digest(&self) -> Digest {
        self.as_bytes().to_digest()
    }
}

impl Digestible for String {
    fn to_digest(&self) -> Digest {
        self.as_bytes().to_digest()
    }
}

macro_rules! impl_digestable_for_numeric {
    ($x: ty) => {
        impl Digestible for $x {
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
        let expect = Digest(*b"\x32\x4d\xcf\x02\x7d\xd4\xa3\x0a\x93\x2c\x44\x1f\x36\x5a\x25\xe8\x6b\x17\x3d\xef\xa4\xb8\xe5\x89\x48\x25\x34\x71\xb8\x1b\x72\xcf");
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
            "\"324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf\""
        );
        let bin = bincode::serialize(&digest).unwrap();
        assert_eq!(
            bin,
            b"\x32\x4d\xcf\x02\x7d\xd4\xa3\x0a\x93\x2c\x44\x1f\x36\x5a\x25\xe8\x6b\x17\x3d\xef\xa4\xb8\xe5\x89\x48\x25\x34\x71\xb8\x1b\x72\xcf",
        );

        assert_eq!(serde_json::from_str::<Digest>(&json).unwrap(), digest);
        assert_eq!(bincode::deserialize::<Digest>(&bin[..]).unwrap(), digest);
    }
}
