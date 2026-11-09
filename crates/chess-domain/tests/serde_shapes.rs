//! GATE-103 rework DF-06: serde-level tests for `GameSetup`, `Seat` and `Personality`.
//! `serde_json` is not an admitted dependency (OQ-17 part 2 is open), so these tests use a
//! small test-only value tree (`Val`) with its own `Serializer` and `Deserializer`. It models
//! the JSON data model used by the protocol: strings, integers, null, objects. It is a stand-in
//! for the JSON codec, not the codec itself (the codec belongs to `chess-app`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;

use chess_domain::{GameSetup, Personality, PlayerKind, Seat, StartPosition};
use common::{pid, spec, strict_hc, training};
use serde::de::value::{Error as DeError, MapDeserializer, StringDeserializer};
use serde::de::{self, IntoDeserializer, Visitor};
use serde::ser::{self, Impossible};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
enum Val {
    Str(String),
    Int(i64),
    Null,
    Map(Vec<(String, Val)>),
}

fn s(text: &str) -> Val {
    Val::Str(text.to_string())
}

fn get<'a>(v: &'a Val, key: &str) -> &'a Val {
    match v {
        Val::Map(pairs) => &pairs.iter().find(|(k, _)| k == key).unwrap().1,
        other => panic!("not a map: {other:?}"),
    }
}

fn without(v: &Val, key: &str) -> Val {
    match v {
        Val::Map(pairs) => Val::Map(pairs.iter().filter(|(k, _)| k != key).cloned().collect()),
        other => panic!("not a map: {other:?}"),
    }
}

fn with(v: &Val, key: &str, value: Val) -> Val {
    match v {
        Val::Map(pairs) => {
            let mut out: Vec<(String, Val)> =
                pairs.iter().filter(|(k, _)| k != key).cloned().collect();
            out.push((key.to_string(), value));
            Val::Map(out)
        }
        other => panic!("not a map: {other:?}"),
    }
}

// ------------------------------------------------------------ deserializer

impl<'de> IntoDeserializer<'de, DeError> for Val {
    type Deserializer = Val;
    fn into_deserializer(self) -> Val {
        self
    }
}

impl<'de> de::Deserializer<'de> for Val {
    type Error = DeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self {
            Val::Str(text) => visitor.visit_string(text),
            Val::Int(n) => visitor.visit_i64(n),
            Val::Null => visitor.visit_unit(),
            Val::Map(pairs) => visitor.visit_map(MapDeserializer::new(pairs.into_iter())),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self {
            Val::Null => visitor.visit_none(),
            other => visitor.visit_some(other),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError> {
        match self {
            Val::Str(text) => {
                let d: StringDeserializer<DeError> = text.into_deserializer();
                visitor.visit_enum(d)
            }
            other => Err(de::Error::custom(format!("not an enum: {other:?}"))),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

fn from_val<T: for<'de> Deserialize<'de>>(v: &Val) -> Result<T, DeError> {
    T::deserialize(v.clone())
}

// ------------------------------------------------------------ serializer

struct ValSer;

struct StructSer(Vec<(String, Val)>);

impl ser::SerializeStruct for StructSer {
    type Ok = Val;
    type Error = DeError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), DeError> {
        self.0.push((key.to_string(), value.serialize(ValSer)?));
        Ok(())
    }
    fn end(self) -> Result<Val, DeError> {
        Ok(Val::Map(self.0))
    }
}

macro_rules! ints {
    ($($f:ident $t:ty),*) => {$(
        fn $f(self, v: $t) -> Result<Val, DeError> {
            Ok(Val::Int(i64::try_from(v).map_err(ser::Error::custom)?))
        }
    )*};
}

macro_rules! unsupported {
    ($($f:ident ($($a:ty),*)),*) => {$(
        fn $f(self, $(_: $a),*) -> Result<Val, DeError> {
            Err(ser::Error::custom("not part of the test model"))
        }
    )*};
}

impl ser::Serializer for ValSer {
    type Ok = Val;
    type Error = DeError;
    type SerializeSeq = Impossible<Val, DeError>;
    type SerializeTuple = Impossible<Val, DeError>;
    type SerializeTupleStruct = Impossible<Val, DeError>;
    type SerializeTupleVariant = Impossible<Val, DeError>;
    type SerializeMap = Impossible<Val, DeError>;
    type SerializeStruct = StructSer;
    type SerializeStructVariant = Impossible<Val, DeError>;

    ints!(serialize_i8 i8, serialize_i16 i16, serialize_i32 i32, serialize_i64 i64,
          serialize_u8 u8, serialize_u16 u16, serialize_u32 u32, serialize_u64 u64);
    unsupported!(
        serialize_bool(bool),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char),
        serialize_bytes(&[u8]),
        serialize_unit_struct(&'static str)
    );

    fn serialize_str(self, v: &str) -> Result<Val, DeError> {
        Ok(Val::Str(v.to_string()))
    }
    fn serialize_none(self) -> Result<Val, DeError> {
        Ok(Val::Null)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<Val, DeError> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<Val, DeError> {
        Ok(Val::Null)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Val, DeError> {
        Ok(Val::Str(variant.to_string()))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<Val, DeError> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Val, DeError> {
        Err(ser::Error::custom("not part of the test model"))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, DeError> {
        Err(ser::Error::custom("not part of the test model"))
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, DeError> {
        Err(ser::Error::custom("not part of the test model"))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, DeError> {
        Err(ser::Error::custom("not part of the test model"))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, DeError> {
        Err(ser::Error::custom("not part of the test model"))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, DeError> {
        Err(ser::Error::custom("not part of the test model"))
    }
    fn serialize_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, DeError> {
        Ok(StructSer(Vec::with_capacity(len)))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, DeError> {
        Err(ser::Error::custom("not part of the test model"))
    }
}

fn to_val<T: Serialize>(t: &T) -> Val {
    t.serialize(ValSer).unwrap()
}

// ------------------------------------------------------------ Seat

#[test]
fn df_06_seat_round_trips_and_has_the_documented_shape() {
    let seat = Seat {
        player: pid(7),
        kind: PlayerKind::Computer,
    };
    let v = to_val(&seat);
    assert_eq!(
        get(&v, "player"),
        &s("00000000-0000-0000-0000-000000000007")
    );
    assert_eq!(get(&v, "kind"), &s("computer"));
    assert_eq!(from_val::<Seat>(&v).unwrap(), seat);
}

#[test]
fn df_06_seat_rejects_unknown_missing_and_integer_fields() {
    let v = to_val(&Seat {
        player: pid(7),
        kind: PlayerKind::Human,
    });
    assert!(from_val::<Seat>(&with(&v, "extra", Val::Int(1))).is_err());
    assert!(from_val::<Seat>(&without(&v, "player")).is_err());
    assert!(from_val::<Seat>(&without(&v, "kind")).is_err());
    // an integer is not an id (D-40)
    assert!(from_val::<Seat>(&with(&v, "player", Val::Int(7))).is_err());
    assert!(from_val::<Seat>(&with(&v, "player", s(""))).is_err());
    assert!(from_val::<Seat>(&with(&v, "kind", s("robot"))).is_err());
    assert!(from_val::<Seat>(&with(&v, "kind", Val::Int(0))).is_err());
}

// ------------------------------------------------------------ GameSetup

#[test]
fn df_06_game_setup_round_trips_untimed_and_timed() {
    for setup in [training(), common::strict_hh(), strict_hc()] {
        let v = to_val(&setup);
        assert_eq!(from_val::<GameSetup>(&v).unwrap(), setup);
    }
    let timed = common::strict_hh();
    let v = to_val(&timed);
    assert_eq!(get(&v, "clock"), &s("300+0"));
    assert_eq!(get(&v, "start"), &Val::Null);
    assert_eq!(get(&v, "matchup"), &s("hh"));
    assert_eq!(get(&v, "mode"), &s("strict"));
    assert_eq!(from_val::<GameSetup>(&v).unwrap().clock, Some(spec(5, 0)));
}

#[test]
fn df_06_game_setup_round_trips_a_custom_start() {
    let fen = "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1";
    let mut setup = training();
    setup.start = StartPosition::from_fen(chess_rules::Fen::parse(fen).unwrap());
    let v = to_val(&setup);
    assert_eq!(get(&v, "start"), &s(fen));
    assert_eq!(from_val::<GameSetup>(&v).unwrap(), setup);
    assert!(from_val::<GameSetup>(&with(&v, "start", s("not a fen"))).is_err());
}

#[test]
fn df_06_game_setup_rejects_unknown_fields() {
    let v = to_val(&training());
    assert!(from_val::<GameSetup>(&with(&v, "start_fen", Val::Null)).is_err());
    assert!(from_val::<GameSetup>(&with(&v, "self_play", s("true"))).is_err());
    // a nested unknown field is rejected too
    let white = with(get(&v, "white"), "extra", Val::Int(1));
    assert!(from_val::<GameSetup>(&with(&v, "white", white)).is_err());
}

#[test]
fn df_06_game_setup_missing_keys() {
    let v = to_val(&training());
    for key in ["game_id", "matchup", "mode", "white", "black"] {
        assert!(
            from_val::<GameSetup>(&without(&v, key)).is_err(),
            "missing {key} must be rejected"
        );
    }
    // OBSERVED: a missing `start` key is the standard start. serde's derive hands an absent
    // field to the field type's `Deserialize` as "none", and `StartPosition` reads
    // `Option<String>`, so `None` becomes `Standard` (lenient; finding in the rework evidence).
    let custom = {
        let mut c = training();
        c.start = StartPosition::from_fen(
            chess_rules::Fen::parse("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap(),
        );
        to_val(&c)
    };
    assert_eq!(
        from_val::<GameSetup>(&without(&custom, "start"))
            .unwrap()
            .start,
        StartPosition::Standard
    );
    // OBSERVED: a missing `clock` key is `None`, an untimed game (lenient; recorded as a finding
    // in the rework evidence, not changed here).
    let no_clock = without(&to_val(&common::strict_hh()), "clock");
    assert_eq!(from_val::<GameSetup>(&no_clock).unwrap().clock, None);
}

#[test]
fn df_06_game_setup_rejects_integer_ids_and_bad_enums() {
    let v = to_val(&training());
    assert!(from_val::<GameSetup>(&with(&v, "game_id", Val::Int(100))).is_err());
    assert!(from_val::<GameSetup>(&with(&v, "game_id", s(""))).is_err());
    assert!(from_val::<GameSetup>(&with(&v, "matchup", s("HC"))).is_err());
    assert!(from_val::<GameSetup>(&with(&v, "mode", s("blitz"))).is_err());
    assert!(from_val::<GameSetup>(&with(&v, "clock", s("05+0"))).is_err());
    assert!(from_val::<GameSetup>(&with(&v, "clock", Val::Int(300))).is_err());
}

// ------------------------------------------------------------ Personality

fn personality() -> Personality {
    Personality::new(10, 8, 500, Some(1500), 100, 2).unwrap()
}

#[test]
fn df_06_personality_round_trips_with_and_without_uci_elo() {
    let with_elo = personality();
    let v = to_val(&with_elo);
    assert_eq!(get(&v, "uci_elo"), &Val::Int(1500));
    assert_eq!(from_val::<Personality>(&v).unwrap(), with_elo);
    let without_elo = Personality::new(3, 4, 100, None, 0, 1).unwrap();
    let v = to_val(&without_elo);
    assert_eq!(get(&v, "uci_elo"), &Val::Null);
    assert_eq!(from_val::<Personality>(&v).unwrap(), without_elo);
}

#[test]
fn df_06_personality_unknown_missing_and_invalid() {
    let v = to_val(&personality());
    assert!(from_val::<Personality>(&with(&v, "extra", Val::Int(1))).is_err());
    for key in [
        "skill_level",
        "depth_limit",
        "movetime_ms",
        "error_permille",
        "error_multipv",
    ] {
        assert!(
            from_val::<Personality>(&without(&v, key)).is_err(),
            "missing {key} must be rejected"
        );
    }
    // `uci_elo` has `#[serde(default)]`: absent means `None`
    let p = from_val::<Personality>(&without(&v, "uci_elo")).unwrap();
    assert_eq!(p.uci_elo, None);
    // validation runs on the way in (`try_from`)
    assert!(from_val::<Personality>(&with(&v, "error_permille", Val::Int(1001))).is_err());
    assert!(from_val::<Personality>(&with(&v, "error_multipv", Val::Int(1))).is_err());
    // wrong types and out-of-range integers
    assert!(from_val::<Personality>(&with(&v, "skill_level", s("10"))).is_err());
    assert!(from_val::<Personality>(&with(&v, "skill_level", Val::Int(256))).is_err());
    assert!(from_val::<Personality>(&with(&v, "skill_level", Val::Int(-1))).is_err());
    assert!(from_val::<Personality>(&with(&v, "depth_limit", Val::Int(i64::MAX))).is_err());
}
