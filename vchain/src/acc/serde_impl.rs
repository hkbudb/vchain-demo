use ark_ec::AffineCurve;
use core::marker::PhantomData;
use serde::{
    de::{Deserializer, Visitor},
    ser::Serializer,
};

pub fn serialize<S: Serializer, C: AffineCurve>(c: &C, s: S) -> Result<S::Ok, S::Error> {
    let mut buf = Vec::<u8>::new();
    c.serialize(&mut buf)
        .map_err(<S::Error as serde::ser::Error>::custom)?;
    if s.is_human_readable() {
        s.serialize_str(&hex::encode(&buf))
    } else {
        s.serialize_bytes(&buf)
    }
}

pub fn deserialize<'de, D: Deserializer<'de>, C: AffineCurve>(d: D) -> Result<C, D::Error> {
    use core::fmt;
    use serde::de::Error as DeError;

    struct HexVisitor<C>(PhantomData<C>);

    impl<'de, C: AffineCurve> Visitor<'de> for HexVisitor<C> {
        type Value = C;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("AffineCurve")
        }

        fn visit_str<E: DeError>(self, value: &str) -> Result<C, E> {
            let data = hex::decode(value).map_err(E::custom)?;
            C::deserialize(&data[..]).map_err(E::custom)
        }
    }

    struct BytesVisitor<C>(PhantomData<C>);

    impl<'de, C: AffineCurve> Visitor<'de> for BytesVisitor<C> {
        type Value = C;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("AffineCurve")
        }

        fn visit_bytes<E: DeError>(self, v: &[u8]) -> Result<C, E> {
            C::deserialize(v).map_err(E::custom)
        }
    }

    if d.is_human_readable() {
        d.deserialize_str(HexVisitor(PhantomData))
    } else {
        d.deserialize_bytes(BytesVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::{G1Affine, G2Affine};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    struct Foo {
        #[serde(with = "super")]
        f1: G1Affine,
        #[serde(with = "super")]
        f2: G2Affine,
    }

    #[test]
    fn test_serde() {
        #[allow(clippy::blacklisted_name)]
        let foo = Foo {
            f1: G1Affine::prime_subgroup_generator(),
            f2: G2Affine::prime_subgroup_generator(),
        };

        let json = serde_json::to_string_pretty(&foo).unwrap();
        let bin = bincode::serialize(&foo).unwrap();

        assert_eq!(serde_json::from_str::<Foo>(&json).unwrap(), foo);
        assert_eq!(bincode::deserialize::<Foo>(&bin[..]).unwrap(), foo);
    }
}
