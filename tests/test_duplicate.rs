//! These tests are meant for the `DuplicateQS` method

use serde::Deserialize;
use serde_querystring::de::{from_bytes, Config};

/// It is a helper struct we use to test primitive types
/// as we don't support anything beside maps/structs at the root level
#[derive(Debug, PartialEq, Deserialize)]
struct Primitive<T> {
    value: T,
}

impl<T> Primitive<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

macro_rules! p {
    ($value:expr, $type: ty) => {
        Primitive::<$type>::new($value)
    };
    ($value:expr) => {
        Primitive::new($value)
    };
}

#[derive(Debug, Deserialize, PartialEq)]
struct Duplicate<'a> {
    #[serde(borrow)]
    foo: &'a str,
    foobar: u32,
    bar: Option<u32>,
    vec: Vec<u32>,
}

#[test]
fn deserialize_duplicate() {
    assert_eq!(
        from_bytes(
            b"foo=bar&foobar=1337&foo=baz&bar=13&vec=1337&vec=11",
            Config::Duplicate
        ),
        Ok(Duplicate {
            foo: "baz",
            foobar: 1337,
            bar: Some(13),
            vec: vec![1337, 11]
        })
    )
}

#[test]
fn deserialize_value() {
    // vector
    assert_eq!(
        from_bytes(b"value=1&value=3&value=1337", Config::Duplicate),
        Ok(p!(1337))
    );

    assert_eq!(
        from_bytes(b"value=1&value=3&value=1337", Config::Duplicate),
        Ok(p!("1337"))
    );
}

#[test]
fn deserialize_sequence() {
    // vector
    assert_eq!(
        from_bytes(b"value=1&value=3&value=1337", Config::Duplicate),
        Ok(p!(vec![1, 3, 1337]))
    );

    // array
    assert_eq!(
        from_bytes(b"value=1&value=3&value=1337", Config::Duplicate),
        Ok(p!([1, 3, 1337]))
    );

    // tuple
    assert_eq!(
        from_bytes(b"value=1&value=3&value=1337", Config::Duplicate),
        Ok(p!((1, 3, 1337)))
    );
    assert_eq!(
        from_bytes(b"value=1&value=3&value=1337", Config::Duplicate),
        Ok(p!((true, "3", 1337)))
    );

    #[derive(Debug, Deserialize, Hash, Eq, PartialEq)]
    enum Side {
        Left,
        Right,
        God,
    }

    // unit enums in sequence
    assert_eq!(
        from_bytes(b"value=God&value=Left&value=Right", Config::Duplicate),
        Ok(p!(vec![Side::God, Side::Left, Side::Right]))
    );
}

#[test]
fn deserialize_decoded_keys() {
    // having different encoded kinds of the string `value` for key
    // `v%61lu%65` `valu%65` `value`
    assert_eq!(
        from_bytes(b"v%61lu%65=1&valu%65=2&value=3", Config::Duplicate),
        Ok(p!(vec!["1", "2", "3"]))
    );
}

#[test]
fn deserialize_invalid_sequence() {
    // array length
    assert!(from_bytes::<Primitive<[usize; 3]>>(
        b"value=1&value=3&value=1337&value=999",
        Config::Duplicate
    )
    .is_err());

    // tuple length
    assert!(from_bytes::<Primitive<(usize, usize, usize)>>(
        b"value=1&value=3&value=1337&value=999",
        Config::Duplicate
    )
    .is_err());

    // tuple value types
    assert!(from_bytes::<Primitive<(&str, usize, &str)>>(
        b"value=foo&value=bar&value=baz",
        Config::Duplicate
    )
    .is_err());
}
