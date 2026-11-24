//! DD-ST-015 `JournalFormat`: the on-disk format of the per-game journal (SDD 3.11.1,
//! architecture 7.6, SF-07 to SF-16). **This is a stored format (G3).** Everything here is the
//! text of those documents, nothing more:
//!
//! - header: magic `CHSJ` (4 bytes), `journal_version: u16` (1), `game_uuid` (16 bytes),
//!   `store_id` (16 bytes), `header_crc32` (covers magic to the end of `store_id`);
//! - record: sync `A5 5A` (2 bytes, not covered by the checksum), `len: u32` (body length),
//!   `crc32: u32` (covers `len`, 4 bytes little-endian, then the body), then the body:
//!   `seq: u64`, `kind: u8`, `wall_ms: i64`, payload (UTF-8 JSON; fields this build does not know are ignored on read, FIND-B-11);
//! - integers little-endian; CRC-32/ISO-HDLC (check value `0xCBF43926` for `123456789`);
//!   a body larger than 65 536 bytes is refused by the writer and read as damage;
//! - kind codes `Start` 1 ... `Voided` 14; code 0 and 15 to 255 are invalid.
//!
//! The payload structs below carry exactly the fields of the SDD 3.11.1 table, in its order.
//! Every nullable field must be **present** (`null` or a value): a missing key is a decode
//! error. The text forms of SF-10 to SF-15 (lower-case hyphenated UUIDs, `clock_spec`
//! `<base>+<increment>`, snake_case reasons, both-or-neither clocks) are enforced at decode and
//! at encode, so the writer cannot write what the reader would refuse.

use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

/// The file magic.
pub const MAGIC: [u8; 4] = *b"CHSJ";
/// `journal_version` of this build (nothing is released; stays 1, SDD 3.11.1).
pub const JOURNAL_VERSION: u16 = 1;
/// Header length: magic, version, game uuid, store id, checksum.
pub const HEADER_LEN: usize = 4 + 2 + 16 + 16 + 4;
/// The two sync bytes that start every record.
pub const SYNC: [u8; 2] = [0xA5, 0x5A];
/// Largest record body (SF-09).
pub const MAX_BODY: usize = 65_536;
/// `seq` (8), `kind` (1), `wall_ms` (8): the smallest body.
pub const FIXED_BODY: usize = 8 + 1 + 8;
/// Sync (2), length (4), checksum (4) before the body.
pub const FRAME_PREFIX: usize = 2 + 4 + 4;

/// CRC-32/ISO-HDLC of `bytes` (SF-08).
pub fn crc32(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

// ---------------------------------------------------------------------------------- kinds

/// The record kinds with their codes (SF-07). A code is never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Start = 1,
    Move = 2,
    Takeback = 3,
    Clock = 4,
    DrawOffer = 5,
    DrawDeclined = 6,
    DrawDropped = 7,
    Pause = 8,
    Resume = 9,
    Suspend = 10,
    SuspendResume = 11,
    GameEnd = 12,
    Abort = 13,
    Voided = 14,
}

impl Kind {
    pub const ALL: [Kind; 14] = [
        Kind::Start,
        Kind::Move,
        Kind::Takeback,
        Kind::Clock,
        Kind::DrawOffer,
        Kind::DrawDeclined,
        Kind::DrawDropped,
        Kind::Pause,
        Kind::Resume,
        Kind::Suspend,
        Kind::SuspendResume,
        Kind::GameEnd,
        Kind::Abort,
        Kind::Voided,
    ];

    pub fn code(self) -> u8 {
        self as u8
    }

    /// `None` for 0 and 15 to 255.
    pub fn from_code(code: u8) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.code() == code)
    }

    /// The payload field names of this kind, in the order of the SDD 3.11.1 table
    /// (`wall_ms` is in the record body, not the payload).
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Kind::Start => &[
                "game",
                "white",
                "black",
                "white_kind",
                "black_kind",
                "matchup",
                "mode",
                "custom_start",
                "start_fen",
                "clock_spec",
                "rated",
                "self_play",
            ],
            Kind::Move => &[
                "ply",
                "uci",
                "think_ms",
                "white_ms",
                "black_ms",
                "san",
                "fen_after",
            ],
            Kind::Takeback => &["to_ply"],
            Kind::Clock => &["white_ms", "black_ms", "running_side"],
            Kind::DrawOffer | Kind::DrawDeclined => &["by"],
            Kind::DrawDropped => &["by", "cause"],
            Kind::Pause | Kind::Resume | Kind::SuspendResume => &[],
            Kind::Suspend => &["cause"],
            Kind::GameEnd => &[
                "result",
                "reason",
                "ended_at",
                "eco",
                "opening_name",
                "analysis_used",
                "content_hash",
                "white_time_used_ms",
                "black_time_used_ms",
                "total_active_ms",
            ],
            Kind::Abort => &["ended_at", "content_hash"],
            Kind::Voided => &["reason", "reset_player", "content_hash", "ended_at"],
        }
    }
}

// ------------------------------------------------------------------------ text-form types

/// A UUID in the one text form of SF-12: lower-case, hyphenated, 36 characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UuidText(pub Uuid);

impl Serialize for UuidText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0.hyphenated())
    }
}

impl<'de> Deserialize<'de> for UuidText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<UuidText, D::Error> {
        let text = String::deserialize(deserializer)?;
        let parsed = Uuid::try_parse(&text).map_err(de::Error::custom)?;
        if parsed.hyphenated().to_string() == text {
            Ok(UuidText(parsed))
        } else {
            Err(de::Error::custom("uuid must be lower-case and hyphenated"))
        }
    }
}

/// `clock_spec` text (SF-11): `<base>+<increment>`, both whole seconds in decimal, no sign, no
/// spaces, no leading zeros (`0` alone is allowed), and inside the limits of `chess-domain`'s
/// `ClockSpec` (base 1 to 86 400 s, increment 0 to 3 600 s): the journal never accepts a clock
/// that the consumer would refuse (FIND-RW1-05). The check **is** `ClockSpec::from_spec_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockText(String);

impl ClockText {
    pub fn parse(text: &str) -> Option<ClockText> {
        chess_domain::ClockSpec::from_spec_text(text)
            .ok()
            .map(|_| ClockText(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for ClockText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ClockText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<ClockText, D::Error> {
        let text = String::deserialize(deserializer)?;
        ClockText::parse(&text).ok_or_else(|| de::Error::custom("bad clock_spec"))
    }
}

/// A key that must be present: `null` or a value, never missing. (Serde's derive reads a
/// missing `Option` field as `None`; with `deserialize_with` a missing key is an error.)
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $text)] $variant),+
        }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }
        }
    };
}

text_enum!(
    /// A side, the store's own `StoredSide` (not a `chess-rules` type).
    Side { White => "white", Black => "black" }
);
text_enum!(
    /// `white_kind` and `black_kind` of `Start`.
    SeatKind { Human => "human", Computer => "computer" }
);
text_enum!(
    /// `DrawDropped.cause`.
    DropCause { EngineFailure => "engine_failure", Suspend => "suspend" }
);
text_enum!(
    /// `Suspend.cause` (SF-21).
    SuspendCause { Detach => "detach", Shutdown => "shutdown", Recovered => "recovered" }
);
text_enum!(
    /// `Voided.reason` (SF-10).
    VoidReason { Reset => "reset", Unreadable => "unreadable" }
);
text_enum!(
    /// `GameEnd.reason` (SF-15).
    EndReason {
        Checkmate => "checkmate",
        Stalemate => "stalemate",
        InsufficientMaterial => "insufficient_material",
        ThreefoldRepetition => "threefold_repetition",
        FiftyMoveRule => "fifty_move_rule",
        TimeForfeit => "time_forfeit",
        TimeoutOpponentCannotMate => "timeout_opponent_cannot_mate",
        Resignation => "resignation",
        DrawAgreed => "draw_agreed",
    }
);

// ---------------------------------------------------------------------------- the payloads

/// `Start` (kind 1). There is no `started_at`: `GAME.started_at` is the record's `wall_ms`
/// (SF-16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartPayload {
    pub game: UuidText,
    pub white: UuidText,
    pub black: UuidText,
    pub white_kind: SeatKind,
    pub black_kind: SeatKind,
    pub matchup: String,
    pub mode: String,
    pub custom_start: bool,
    /// JSON `null` exactly for the standard start (SF-02).
    #[serde(deserialize_with = "nullable")]
    pub start_fen: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub clock_spec: Option<ClockText>,
    pub rated: bool,
    pub self_play: bool,
}

/// `Move` (kind 2). `ply` is the number of plies already played (the first move has `ply` 0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovePayload {
    pub ply: u32,
    pub uci: String,
    pub think_ms: u64,
    #[serde(deserialize_with = "nullable")]
    pub white_ms: Option<u64>,
    #[serde(deserialize_with = "nullable")]
    pub black_ms: Option<u64>,
    pub san: String,
    pub fen_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakebackPayload {
    pub to_ply: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockPayload {
    #[serde(deserialize_with = "nullable")]
    pub white_ms: Option<u64>,
    #[serde(deserialize_with = "nullable")]
    pub black_ms: Option<u64>,
    #[serde(deserialize_with = "nullable")]
    pub running_side: Option<Side>,
}

/// `DrawOffer` and `DrawDeclined`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawByPayload {
    pub by: Side,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawDroppedPayload {
    pub by: Side,
    pub cause: DropCause,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspendPayload {
    pub cause: SuspendCause,
}

/// The payload `{}` of `Pause`, `Resume` and `SuspendResume`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyPayload {}

/// `GameEnd` (kind 12): the finalize inputs. `content_hash` is stored as text and not
/// interpreted by this crate (D-42, D-68).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameEndPayload {
    pub result: String,
    pub reason: EndReason,
    pub ended_at: i64,
    #[serde(deserialize_with = "nullable")]
    pub eco: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub opening_name: Option<String>,
    pub analysis_used: bool,
    pub content_hash: String,
    pub white_time_used_ms: u64,
    pub black_time_used_ms: u64,
    pub total_active_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbortPayload {
    pub ended_at: i64,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoidedPayload {
    pub reason: VoidReason,
    #[serde(deserialize_with = "nullable")]
    pub reset_player: Option<UuidText>,
    pub content_hash: String,
    pub ended_at: i64,
}

/// The payload of one record, by kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    Start(StartPayload),
    Move(MovePayload),
    Takeback(TakebackPayload),
    Clock(ClockPayload),
    DrawOffer(DrawByPayload),
    DrawDeclined(DrawByPayload),
    DrawDropped(DrawDroppedPayload),
    Pause,
    Resume,
    Suspend(SuspendPayload),
    SuspendResume,
    GameEnd(GameEndPayload),
    Abort(AbortPayload),
    Voided(VoidedPayload),
}

/// Why a payload or a record was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatError {
    /// The body would exceed 65 536 bytes (SF-09).
    TooLarge,
    /// The payload is not valid JSON of the kind's shape or breaks a text form.
    BadPayload,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FormatError::TooLarge => "journal record too large",
            FormatError::BadPayload => "journal payload refused",
        })
    }
}

impl std::error::Error for FormatError {}

/// The largest value of a `u64` field (CH-4): the store keeps these as signed 64-bit integers,
/// so a larger value could be journaled but never applied. Refused at encode and at decode.
pub const MAX_COUNT: u64 = u64::MAX >> 1;

/// The database code of a matchup (`hh`, `hc`, `cc`; the table CHECK of `GAME.matchup`): the
/// only text `Start.matchup` may hold. A human against a computer is `hc` whichever side is
/// white.
pub(crate) fn matchup_code(white: SeatKind, black: SeatKind) -> &'static str {
    match (white, black) {
        (SeatKind::Human, SeatKind::Human) => "hh",
        (SeatKind::Computer, SeatKind::Computer) => "cc",
        _ => "hc",
    }
}

/// The record-level rule that the payload alone cannot state: `Start.wall_ms` becomes
/// `GAME.started_at` (SF-16), so it must be above zero (FIND-D3R-07).
fn check_record(wall_ms: i64, payload: &Payload) -> Result<(), FormatError> {
    if matches!(payload, Payload::Start(_)) && wall_ms <= 0 {
        return Err(FormatError::BadPayload);
    }
    Ok(())
}

fn pair_ok(white: Option<u64>, black: Option<u64>) -> bool {
    white.is_some() == black.is_some()
}

fn in_range(values: &[Option<u64>]) -> bool {
    values.iter().all(|v| v.is_none_or(|v| v <= MAX_COUNT))
}

impl Payload {
    pub fn kind(&self) -> Kind {
        match self {
            Payload::Start(_) => Kind::Start,
            Payload::Move(_) => Kind::Move,
            Payload::Takeback(_) => Kind::Takeback,
            Payload::Clock(_) => Kind::Clock,
            Payload::DrawOffer(_) => Kind::DrawOffer,
            Payload::DrawDeclined(_) => Kind::DrawDeclined,
            Payload::DrawDropped(_) => Kind::DrawDropped,
            Payload::Pause => Kind::Pause,
            Payload::Resume => Kind::Resume,
            Payload::Suspend(_) => Kind::Suspend,
            Payload::SuspendResume => Kind::SuspendResume,
            Payload::GameEnd(_) => Kind::GameEnd,
            Payload::Abort(_) => Kind::Abort,
            Payload::Voided(_) => Kind::Voided,
        }
    }

    /// The rules the derive cannot state: `white_ms` and `black_ms` are both `null` or both
    /// numbers (SF-14), no `u64` field is above `i64::MAX` (CH-4), `ended_at` of `GameEnd`,
    /// `Abort` and `Voided` is above zero (D3 ruling 2026-11-24, as the store requires), and
    /// `Start.matchup` is the code (`hh`, `hc`, `cc`) of the two seat kinds. Run at encode and
    /// at decode, so the writer cannot write what the reader refuses.
    fn validate(&self) -> Result<(), FormatError> {
        let ok = match self {
            Payload::Move(m) => {
                pair_ok(m.white_ms, m.black_ms)
                    && in_range(&[Some(m.think_ms), m.white_ms, m.black_ms])
            }
            Payload::Clock(c) => {
                pair_ok(c.white_ms, c.black_ms) && in_range(&[c.white_ms, c.black_ms])
            }
            Payload::GameEnd(e) => {
                e.ended_at > 0
                    && in_range(&[
                        Some(e.white_time_used_ms),
                        Some(e.black_time_used_ms),
                        Some(e.total_active_ms),
                    ])
            }
            Payload::Abort(a) => a.ended_at > 0,
            Payload::Voided(v) => v.ended_at > 0,
            Payload::Start(s) => s.matchup == matchup_code(s.white_kind, s.black_kind),
            _ => true,
        };
        if ok {
            Ok(())
        } else {
            Err(FormatError::BadPayload)
        }
    }

    /// The JSON bytes of the payload.
    pub fn to_json(&self) -> Result<Vec<u8>, FormatError> {
        self.validate()?;
        let bytes = match self {
            Payload::Start(p) => serde_json::to_vec(p),
            Payload::Move(p) => serde_json::to_vec(p),
            Payload::Takeback(p) => serde_json::to_vec(p),
            Payload::Clock(p) => serde_json::to_vec(p),
            Payload::DrawOffer(p) | Payload::DrawDeclined(p) => serde_json::to_vec(p),
            Payload::DrawDropped(p) => serde_json::to_vec(p),
            Payload::Pause | Payload::Resume | Payload::SuspendResume => {
                serde_json::to_vec(&EmptyPayload {})
            }
            Payload::Suspend(p) => serde_json::to_vec(p),
            Payload::GameEnd(p) => serde_json::to_vec(p),
            Payload::Abort(p) => serde_json::to_vec(p),
            Payload::Voided(p) => serde_json::to_vec(p),
        };
        bytes.map_err(|_| FormatError::BadPayload)
    }

    /// Decodes the payload bytes of a record of `kind`. Missing fields, wrong types, values above
    /// `i64::MAX` and broken text forms are `BadPayload`; unknown fields are ignored; the function never panics.
    pub fn from_json(kind: Kind, bytes: &[u8]) -> Result<Payload, FormatError> {
        fn parse<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, FormatError> {
            // FIND-RW1-04: serde reads a JSON array as the positional form of a struct; a
            // payload is an object, so anything else is refused before it is parsed.
            if bytes.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'{') {
                return Err(FormatError::BadPayload);
            }
            serde_json::from_slice(bytes).map_err(|_| FormatError::BadPayload)
        }
        let payload = match kind {
            Kind::Start => Payload::Start(parse(bytes)?),
            Kind::Move => Payload::Move(parse(bytes)?),
            Kind::Takeback => Payload::Takeback(parse(bytes)?),
            Kind::Clock => Payload::Clock(parse(bytes)?),
            Kind::DrawOffer => Payload::DrawOffer(parse(bytes)?),
            Kind::DrawDeclined => Payload::DrawDeclined(parse(bytes)?),
            Kind::DrawDropped => Payload::DrawDropped(parse(bytes)?),
            Kind::Pause => parse::<EmptyPayload>(bytes).map(|_| Payload::Pause)?,
            Kind::Resume => parse::<EmptyPayload>(bytes).map(|_| Payload::Resume)?,
            Kind::Suspend => Payload::Suspend(parse(bytes)?),
            Kind::SuspendResume => parse::<EmptyPayload>(bytes).map(|_| Payload::SuspendResume)?,
            Kind::GameEnd => Payload::GameEnd(parse(bytes)?),
            Kind::Abort => Payload::Abort(parse(bytes)?),
            Kind::Voided => Payload::Voided(parse(bytes)?),
        };
        payload.validate()?;
        Ok(payload)
    }
}

/// One record: `seq`, `wall_ms` and the payload (the kind follows from the payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRecord {
    pub seq: u64,
    pub wall_ms: i64,
    pub payload: Payload,
}

impl JournalRecord {
    pub fn kind(&self) -> Kind {
        self.payload.kind()
    }
}

// ---------------------------------------------------------------------------- byte access

/// `len` bytes of `buf` from `at`, if they are all there.
pub(crate) fn take(buf: &[u8], at: usize, len: usize) -> Option<&[u8]> {
    buf.get(at..at.checked_add(len)?)
}

pub(crate) fn le_u16(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(take(buf, at, 2)?.try_into().ok()?))
}

pub(crate) fn le_u32(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(take(buf, at, 4)?.try_into().ok()?))
}

pub(crate) fn le_u64(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(take(buf, at, 8)?.try_into().ok()?))
}

pub(crate) fn le_i64(buf: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(take(buf, at, 8)?.try_into().ok()?))
}

// ----------------------------------------------------------------------------------- header

/// The decoded header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub version: u16,
    pub game: Uuid,
    pub store: Uuid,
}

/// Why a header was refused (all of them are `Unreadable` to the caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    Short,
    BadMagic,
    BadChecksum,
}

/// The 42 header bytes of a journal of this build.
pub fn encode_header(game: Uuid, store: Uuid) -> [u8; HEADER_LEN] {
    let mut out = [0u8; HEADER_LEN];
    let mut at = 0;
    for part in [
        &MAGIC[..],
        &JOURNAL_VERSION.to_le_bytes()[..],
        game.as_bytes(),
        store.as_bytes(),
    ] {
        if let Some(slot) = out.get_mut(at..at + part.len()) {
            slot.copy_from_slice(part);
        }
        at += part.len();
    }
    let crc = crc32(out.get(..HEADER_LEN - 4).unwrap_or(&[]));
    if let Some(slot) = out.get_mut(HEADER_LEN - 4..) {
        slot.copy_from_slice(&crc.to_le_bytes());
    }
    out
}

/// Reads the header at the start of `bytes`. The version is returned as found; the caller
/// compares it with `JOURNAL_VERSION`.
pub fn decode_header(bytes: &[u8]) -> Result<Header, HeaderError> {
    let head = take(bytes, 0, HEADER_LEN).ok_or(HeaderError::Short)?;
    if take(head, 0, 4) != Some(&MAGIC[..]) {
        return Err(HeaderError::BadMagic);
    }
    let stored = le_u32(head, HEADER_LEN - 4).ok_or(HeaderError::Short)?;
    if crc32(head.get(..HEADER_LEN - 4).unwrap_or(&[])) != stored {
        return Err(HeaderError::BadChecksum);
    }
    let uuid_at = |at: usize| {
        take(head, at, 16)
            .and_then(|b| <[u8; 16]>::try_from(b).ok())
            .map(Uuid::from_bytes)
    };
    Ok(Header {
        version: le_u16(head, 4).ok_or(HeaderError::Short)?,
        game: uuid_at(6).ok_or(HeaderError::Short)?,
        store: uuid_at(22).ok_or(HeaderError::Short)?,
    })
}

// ------------------------------------------------------------------------------- records

/// The bytes of one record: sync, `len`, checksum, body. Refuses a body above 65 536 bytes.
pub fn encode_record(record: &JournalRecord) -> Result<Vec<u8>, FormatError> {
    if record.seq > MAX_COUNT {
        return Err(FormatError::BadPayload);
    }
    check_record(record.wall_ms, &record.payload)?;
    let payload = record.payload.to_json()?;
    let body_len = FIXED_BODY + payload.len();
    if body_len > MAX_BODY {
        return Err(FormatError::TooLarge);
    }
    let len = u32::try_from(body_len).map_err(|_| FormatError::TooLarge)?;
    let mut body = Vec::with_capacity(body_len);
    body.extend_from_slice(&record.seq.to_le_bytes());
    body.push(record.kind().code());
    body.extend_from_slice(&record.wall_ms.to_le_bytes());
    body.extend_from_slice(&payload);
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&len.to_le_bytes());
    hasher.update(&body);
    let mut out = Vec::with_capacity(FRAME_PREFIX + body_len);
    out.extend_from_slice(&SYNC);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&hasher.finalize().to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Why a frame at some position is not a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameError {
    /// Fewer bytes than a frame prefix.
    Short,
    /// The sync bytes are wrong.
    Sync,
    /// `len` is above 65 536, below the fixed fields, or past the end of the data.
    Len,
    Crc,
    /// The kind byte is 0 or above 14.
    Kind,
}

/// A frame whose sync, length, checksum and kind are right. The payload is not decoded.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame<'a> {
    /// Bytes from the sync to the end of the body.
    pub total: usize,
    pub seq: u64,
    pub kind: Kind,
    pub wall_ms: i64,
    pub payload: &'a [u8],
}

pub(crate) fn read_frame(buf: &[u8], pos: usize) -> Result<Frame<'_>, FrameError> {
    let prefix = take(buf, pos, FRAME_PREFIX).ok_or(FrameError::Short)?;
    if take(prefix, 0, 2) != Some(&SYNC[..]) {
        return Err(FrameError::Sync);
    }
    let len_raw = le_u32(prefix, 2).ok_or(FrameError::Short)?;
    let stored = le_u32(prefix, 6).ok_or(FrameError::Short)?;
    let len = usize::try_from(len_raw).map_err(|_| FrameError::Len)?;
    if !(FIXED_BODY..=MAX_BODY).contains(&len) {
        return Err(FrameError::Len);
    }
    let body = take(buf, pos + FRAME_PREFIX, len).ok_or(FrameError::Len)?;
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&len_raw.to_le_bytes());
    hasher.update(body);
    if hasher.finalize() != stored {
        return Err(FrameError::Crc);
    }
    let kind = body
        .get(8)
        .copied()
        .and_then(Kind::from_code)
        .ok_or(FrameError::Kind)?;
    Ok(Frame {
        total: FRAME_PREFIX + len,
        seq: le_u64(body, 0).ok_or(FrameError::Len)?,
        kind,
        wall_ms: le_i64(body, 9).ok_or(FrameError::Len)?,
        payload: body.get(FIXED_BODY..).ok_or(FrameError::Len)?,
    })
}

/// Decodes the record of a frame; the payload must be valid for its kind.
pub(crate) fn decode_frame(frame: &Frame<'_>) -> Result<JournalRecord, FormatError> {
    if frame.seq > MAX_COUNT {
        return Err(FormatError::BadPayload);
    }
    let payload = Payload::from_json(frame.kind, frame.payload)?;
    check_record(frame.wall_ms, &payload)?;
    Ok(JournalRecord {
        seq: frame.seq,
        wall_ms: frame.wall_ms,
        payload,
    })
}

/// Decodes one record from the start of `bytes` (for tests and tools). Returns the record and
/// the number of bytes it occupies.
pub fn decode_record(bytes: &[u8]) -> Result<(JournalRecord, usize), FormatError> {
    let frame = read_frame(bytes, 0).map_err(|_| FormatError::BadPayload)?;
    Ok((decode_frame(&frame)?, frame.total))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UT-ST-081: the check value of the algorithm (SF-08).
    #[test]
    fn crc32_check_value_of_iso_hdlc() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn kind_codes_are_those_of_sf07_and_zero_and_15_up_are_invalid() {
        let expected = [
            ("Start", 1),
            ("Move", 2),
            ("Takeback", 3),
            ("Clock", 4),
            ("DrawOffer", 5),
            ("DrawDeclined", 6),
            ("DrawDropped", 7),
            ("Pause", 8),
            ("Resume", 9),
            ("Suspend", 10),
            ("SuspendResume", 11),
            ("GameEnd", 12),
            ("Abort", 13),
            ("Voided", 14),
        ];
        for (kind, (name, code)) in Kind::ALL.into_iter().zip(expected) {
            assert_eq!(format!("{kind:?}"), name);
            assert_eq!(kind.code(), code);
            assert_eq!(Kind::from_code(code), Some(kind));
        }
        assert_eq!(Kind::from_code(0), None);
        for code in 15..=255u8 {
            assert_eq!(Kind::from_code(code), None, "{code}");
        }
    }

    #[test]
    fn the_header_has_the_layout_and_checksum_of_the_sdd() {
        let game = Uuid::from_u128(0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10);
        let store = Uuid::from_u128(0xf0e0_d0c0_b0a0_9080_7060_5040_3020_1000);
        let bytes = encode_header(game, store);
        assert_eq!(bytes.len(), 42);
        assert_eq!(&bytes[..4], b"CHSJ");
        assert_eq!(&bytes[4..6], &[1, 0], "version 1 little-endian");
        assert_eq!(&bytes[6..22], game.as_bytes());
        assert_eq!(&bytes[22..38], store.as_bytes());
        assert_eq!(&bytes[38..], &crc32(&bytes[..38]).to_le_bytes());
        let header = decode_header(&bytes).unwrap();
        assert_eq!(
            header,
            Header {
                version: 1,
                game,
                store
            }
        );
    }

    #[test]
    fn a_damaged_header_is_refused_for_each_reason() {
        let good = encode_header(Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(decode_header(&good[..41]), Err(HeaderError::Short));
        let mut magic = good;
        magic[0] = b'X';
        assert_eq!(decode_header(&magic), Err(HeaderError::BadMagic));
        let mut crc = good;
        crc[10] ^= 1;
        assert_eq!(decode_header(&crc), Err(HeaderError::BadChecksum));
        let mut stored = good;
        stored[41] ^= 1;
        assert_eq!(decode_header(&stored), Err(HeaderError::BadChecksum));
    }

    fn sample_move(seq: u64) -> JournalRecord {
        JournalRecord {
            seq,
            wall_ms: -5,
            payload: Payload::Move(MovePayload {
                ply: 0,
                uci: "e2e4".into(),
                think_ms: 12,
                white_ms: Some(300_000),
                black_ms: Some(299_000),
                san: "e4".into(),
                fen_after: "x".into(),
            }),
        }
    }

    #[test]
    fn the_record_frame_has_the_layout_and_the_checksum_covers_len_and_body_only() {
        let bytes = encode_record(&sample_move(0x0102)).unwrap();
        assert_eq!(&bytes[..2], &[0xA5, 0x5A]);
        let len = u32::from_le_bytes(bytes[2..6].try_into().unwrap());
        assert_eq!(usize::try_from(len).unwrap(), bytes.len() - 10);
        let mut covered = bytes[2..6].to_vec();
        covered.extend_from_slice(&bytes[10..]);
        assert_eq!(&bytes[6..10], &crc32(&covered).to_le_bytes());
        // body: seq u64, kind u8, wall_ms i64, JSON
        assert_eq!(&bytes[10..18], &0x0102u64.to_le_bytes());
        assert_eq!(bytes[18], 2);
        assert_eq!(&bytes[19..27], &(-5i64).to_le_bytes());
        assert_eq!(bytes[27], b'{');
        // The marker is not covered: changing it leaves the checksum valid but the frame bad.
        let mut other = bytes.clone();
        other[0] = 0;
        assert_eq!(read_frame(&other, 0).unwrap_err(), FrameError::Sync);
        let (decoded, used) = decode_record(&bytes).unwrap();
        assert_eq!(decoded, sample_move(0x0102));
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn a_body_above_65536_bytes_is_refused_by_the_writer_and_the_limit_is_exact() {
        let record = |san_len: usize| JournalRecord {
            seq: 1,
            wall_ms: 0,
            payload: Payload::Move(MovePayload {
                ply: 0,
                uci: "a".into(),
                think_ms: 0,
                white_ms: None,
                black_ms: None,
                san: "s".repeat(san_len),
                fen_after: "f".into(),
            }),
        };
        let base = FIXED_BODY + record(0).payload.to_json().unwrap().len();
        let fit = MAX_BODY - base;
        let ok = encode_record(&record(fit)).unwrap();
        assert_eq!(ok.len(), FRAME_PREFIX + MAX_BODY);
        assert!(read_frame(&ok, 0).is_ok(), "exactly 65 536 is allowed");
        assert_eq!(
            encode_record(&record(fit + 1)).unwrap_err(),
            FormatError::TooLarge
        );
        // A length field above the limit is damage, not a record.
        let mut forged = ok.clone();
        forged[2..6].copy_from_slice(&65_537u32.to_le_bytes());
        assert_eq!(read_frame(&forged, 0).unwrap_err(), FrameError::Len);
    }

    #[test]
    fn frames_that_are_not_records_are_classified() {
        let good = encode_record(&sample_move(1)).unwrap();
        assert_eq!(read_frame(&good[..9], 0).unwrap_err(), FrameError::Short);
        assert_eq!(
            read_frame(&good[..good.len() - 1], 0).unwrap_err(),
            FrameError::Len
        );
        let mut crc = good.clone();
        let last = crc.len() - 1;
        crc[last] ^= 0xFF;
        assert_eq!(read_frame(&crc, 0).unwrap_err(), FrameError::Crc);
        // kind 0 and kind 15 with a correct checksum are invalid kinds.
        for bad in [0u8, 15, 255] {
            let mut forged = good.clone();
            forged[18] = bad;
            let mut covered = forged[2..6].to_vec();
            covered.extend_from_slice(&forged[10..]);
            forged[6..10].copy_from_slice(&crc32(&covered).to_le_bytes());
            assert_eq!(read_frame(&forged, 0).unwrap_err(), FrameError::Kind);
        }
        // len below the fixed fields.
        let mut short = good.clone();
        short[2..6].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(read_frame(&short, 0).unwrap_err(), FrameError::Len);
    }

    fn start() -> StartPayload {
        StartPayload {
            game: UuidText(Uuid::from_u128(1)),
            white: UuidText(Uuid::from_u128(2)),
            black: UuidText(Uuid::from_u128(3)),
            white_kind: SeatKind::Human,
            black_kind: SeatKind::Computer,
            matchup: "hc".into(),
            mode: "strict".into(),
            custom_start: false,
            start_fen: None,
            clock_spec: ClockText::parse("300+2"),
            rated: true,
            self_play: false,
        }
    }

    /// FIND-RW1-04, one case per kind: the positional array form of a real payload (its values
    /// in field order), an empty array and `null` are refused; the object itself decodes.
    #[test]
    fn every_kind_refuses_the_array_form_of_its_own_payload() {
        for payload in one_of_each() {
            let kind = payload.kind();
            let object = payload.to_json().unwrap();
            assert_eq!(
                Payload::from_json(kind, &object).unwrap(),
                payload,
                "{kind:?}"
            );
            let serde_json::Value::Object(map) = serde_json::from_slice(&object).unwrap() else {
                panic!("{kind:?}: not an object");
            };
            let array = serde_json::Value::Array(map.values().cloned().collect());
            for text in [
                serde_json::to_string(&array).unwrap(),
                "[]".to_owned(),
                "null".to_owned(),
            ] {
                assert!(
                    Payload::from_json(kind, text.as_bytes()).is_err(),
                    "{kind:?} {text}"
                );
            }
        }
    }

    /// One payload of every kind.
    pub(crate) fn one_of_each() -> Vec<Payload> {
        vec![
            Payload::Start(start()),
            Payload::Move(MovePayload {
                ply: 3,
                uci: "e7e5".into(),
                think_ms: 900,
                white_ms: None,
                black_ms: None,
                san: "e5".into(),
                fen_after: "f".into(),
            }),
            Payload::Takeback(TakebackPayload { to_ply: 2 }),
            Payload::Clock(ClockPayload {
                white_ms: Some(1),
                black_ms: Some(2),
                running_side: Some(Side::Black),
            }),
            Payload::DrawOffer(DrawByPayload { by: Side::White }),
            Payload::DrawDeclined(DrawByPayload { by: Side::Black }),
            Payload::DrawDropped(DrawDroppedPayload {
                by: Side::White,
                cause: DropCause::EngineFailure,
            }),
            Payload::Pause,
            Payload::Resume,
            Payload::Suspend(SuspendPayload {
                cause: SuspendCause::Recovered,
            }),
            Payload::SuspendResume,
            Payload::GameEnd(GameEndPayload {
                result: "1-0".into(),
                reason: EndReason::TimeoutOpponentCannotMate,
                ended_at: 7,
                eco: None,
                opening_name: Some("Ruy Lopez".into()),
                analysis_used: false,
                content_hash: "sha256:v1:abc".into(),
                white_time_used_ms: 1,
                black_time_used_ms: 2,
                total_active_ms: 3,
            }),
            Payload::Abort(AbortPayload {
                ended_at: 8,
                content_hash: "sha256:v1:def".into(),
            }),
            Payload::Voided(VoidedPayload {
                reason: VoidReason::Reset,
                reset_player: Some(UuidText(Uuid::from_u128(9))),
                content_hash: "sha256:v1:0".into(),
                ended_at: 9,
            }),
        ]
    }

    /// UT-ST-070: a record of every kind round-trips, and the payload field names equal the
    /// table of SDD 3.11.1 (names and order).
    #[test]
    fn ut_st_070_every_kind_round_trips_and_has_exactly_the_table_fields() {
        let all = one_of_each();
        assert_eq!(all.len(), Kind::ALL.len());
        for (seq, payload) in (1u64..).zip(all) {
            let kind = payload.kind();
            let record = JournalRecord {
                seq,
                wall_ms: 1_000 + i64::try_from(seq).unwrap(),
                payload,
            };
            let bytes = encode_record(&record).unwrap();
            let (back, used) = decode_record(&bytes).unwrap();
            assert_eq!(back, record, "{kind:?}");
            assert_eq!(used, bytes.len());
            let json: serde_json::Value =
                serde_json::from_slice(&record.payload.to_json().unwrap()).unwrap();
            let names: Vec<&str> = json
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            let mut sorted_expected = kind.fields().to_vec();
            sorted_expected.sort_unstable();
            let mut sorted_names = names.clone();
            sorted_names.sort_unstable();
            assert_eq!(sorted_names, sorted_expected, "{kind:?}");
        }
    }

    #[test]
    fn payload_json_keys_are_written_in_table_order_and_empty_payloads_are_braces() {
        for payload in one_of_each() {
            let text = String::from_utf8(payload.to_json().unwrap()).unwrap();
            let mut at = 0;
            for name in payload.kind().fields() {
                let found = text[at..].find(&format!("\"{name}\":")).unwrap();
                at += found;
            }
        }
        for payload in [Payload::Pause, Payload::Resume, Payload::SuspendResume] {
            assert_eq!(payload.to_json().unwrap(), b"{}");
        }
        // Null fields are written as null, not left out.
        let text = String::from_utf8(Payload::Start(start()).to_json().unwrap()).unwrap();
        assert!(text.contains("\"start_fen\":null"), "{text}");
    }

    #[test]
    fn missing_fields_and_wrong_types_are_decode_errors() {
        let bad = |kind: Kind, json: &str| Payload::from_json(kind, json.as_bytes()).is_err();
        assert!(bad(Kind::Takeback, r"{}"));
        assert!(bad(Kind::Takeback, r#"{"to_ply":-1}"#));
        assert!(bad(Kind::Takeback, r#"{"to_ply":1.5}"#));
        assert!(bad(Kind::Takeback, r#"{"to_ply":"1"}"#));
        assert!(bad(Kind::Pause, "null"));
        assert!(bad(Kind::Clock, r#"{"white_ms":1,"black_ms":2}"#)); // running_side missing
        assert!(bad(Kind::Abort, r#"{"ended_at":1}"#));
        assert!(
            Payload::from_json(Kind::Takeback, &[0xFF, 0xFE]).is_err(),
            "not UTF-8"
        );
        assert!(Payload::from_json(Kind::Pause, b"{}").is_ok());
    }

    #[test]
    fn text_forms_of_sf10_to_sf15_are_enforced() {
        let uuid = "0000000a-0000-0000-0000-00000000000b";
        let voided = |reason: &str, player: &str| {
            format!(
                r#"{{"reason":"{reason}","reset_player":{player},"content_hash":"h","ended_at":1}}"#
            )
        };
        let ok = voided("reset", &format!("\"{uuid}\""));
        assert!(Payload::from_json(Kind::Voided, ok.as_bytes()).is_ok());
        assert!(Payload::from_json(Kind::Voided, voided("reset", "null").as_bytes()).is_ok());
        // SF-10: only reset and unreadable.
        assert!(Payload::from_json(Kind::Voided, voided("Reset", "null").as_bytes()).is_err());
        assert!(
            Payload::from_json(Kind::Voided, voided("voided by reset", "null").as_bytes()).is_err()
        );
        // SF-12: lower case, hyphenated, 36 characters.
        for bad in [
            uuid.to_uppercase(),
            uuid.replace('-', ""),
            format!("{{{uuid}}}"),
            format!("urn:uuid:{uuid}"),
            uuid[..35].to_owned(),
        ] {
            let json = voided("reset", &format!("\"{bad}\""));
            assert!(
                Payload::from_json(Kind::Voided, json.as_bytes()).is_err(),
                "{bad}"
            );
        }
        // SF-11: clock_spec forms.
        for good in ["300+0", "180+2", "1+0", "86400+0", "1+3600", "86400+3600"] {
            assert!(ClockText::parse(good).is_some(), "{good}");
        }
        for bad in [
            "5",
            "05+0",
            "5+00",
            "+5",
            "5+",
            "-5+0",
            "5+ 0",
            " 5+0",
            "5 +0",
            "5+0 ",
            "5+-1",
            "1+2+3",
            "",
            "a+b",
            "99999999999999999999+0",
            // FIND-RW1-05: what `ClockSpec` refuses (base 1 to 86 400 s, increment 0 to 3 600 s),
            // and the u32 and u64 overflows.
            "0+0",
            "0+10",
            "86401+0",
            "1+3601",
            "4294967296+0",
            "1+4294967296",
            "18446744073709551615+18446744073709551615",
        ] {
            assert!(ClockText::parse(bad).is_none(), "{bad:?}");
        }
        // The text form agrees with the domain on every limit, edge by edge.
        for base in [0u32, 1, 2, 86_399, 86_400, 86_401] {
            for inc in [0u32, 1, 3_599, 3_600, 3_601] {
                let text = format!("{base}+{inc}");
                assert_eq!(
                    ClockText::parse(&text).is_some(),
                    chess_domain::ClockSpec::from_seconds(base, inc).is_ok(),
                    "{text}"
                );
            }
        }
        // FIND-RW1-04: a payload is a JSON object, never the positional array form.
        for (kind, array) in [
            (Kind::Pause, "[]"),
            (Kind::Resume, "[]"),
            (Kind::SuspendResume, "[]"),
            (Kind::Takeback, "[1]"),
            (Kind::DrawOffer, r#"["white"]"#),
            (Kind::Clock, "[1,2,null]"),
            (Kind::Abort, r#"[1,"h"]"#),
            (Kind::Pause, "null"),
            (Kind::Pause, "1"),
            (Kind::Pause, r#""{}""#),
        ] {
            assert!(
                Payload::from_json(kind, array.as_bytes()).is_err(),
                "{kind:?} {array}"
            );
        }
        // Leading white space before the object is still an object.
        assert!(Payload::from_json(Kind::Pause, b" \n{}").is_ok());
        // SF-14: both clocks null or both numbers.
        assert!(
            Payload::from_json(
                Kind::Clock,
                br#"{"white_ms":1,"black_ms":null,"running_side":null}"#
            )
            .is_err()
        );
        assert!(
            Payload::from_json(
                Kind::Clock,
                br#"{"white_ms":null,"black_ms":2,"running_side":null}"#
            )
            .is_err()
        );
        assert!(
            Payload::from_json(
                Kind::Clock,
                br#"{"white_ms":null,"black_ms":null,"running_side":"white"}"#
            )
            .is_ok()
        );
        let half = Payload::Clock(ClockPayload {
            white_ms: Some(1),
            black_ms: None,
            running_side: None,
        });
        assert_eq!(
            half.to_json().unwrap_err(),
            FormatError::BadPayload,
            "the writer refuses it too"
        );
        // SF-15: reasons.
        let end = |reason: &str| {
            format!(
                r#"{{"result":"1-0","reason":"{reason}","ended_at":1,"eco":null,"opening_name":null,"analysis_used":false,"content_hash":"h","white_time_used_ms":0,"black_time_used_ms":0,"total_active_ms":0}}"#
            )
        };
        for reason in [
            "checkmate",
            "stalemate",
            "insufficient_material",
            "threefold_repetition",
            "fifty_move_rule",
            "time_forfeit",
            "timeout_opponent_cannot_mate",
            "resignation",
            "draw_agreed",
        ] {
            assert!(
                Payload::from_json(Kind::GameEnd, end(reason).as_bytes()).is_ok(),
                "{reason}"
            );
        }
        for reason in ["Checkmate", "mate", "draw", "timeout", ""] {
            assert!(
                Payload::from_json(Kind::GameEnd, end(reason).as_bytes()).is_err(),
                "{reason}"
            );
        }
        // Suspend.cause (SF-21) and DrawDropped.cause.
        assert!(Payload::from_json(Kind::Suspend, br#"{"cause":"detach"}"#).is_ok());
        assert!(Payload::from_json(Kind::Suspend, br#"{"cause":"lease_lost"}"#).is_err());
        assert!(
            Payload::from_json(Kind::DrawDropped, br#"{"by":"white","cause":"timeout"}"#).is_err()
        );
        assert!(Payload::from_json(Kind::DrawOffer, br#"{"by":"red"}"#).is_err());
    }

    /// FIND-B-11 B (Brian, 2026-11-22): decode ignores fields it does not know, for every kind
    /// (a newer writer may add fields without a `journal_version` bump).
    #[test]
    fn unknown_fields_are_ignored_on_read_for_every_kind() {
        for payload in one_of_each() {
            let kind = payload.kind();
            let mut value: serde_json::Value =
                serde_json::from_slice(&payload.to_json().unwrap()).unwrap();
            let object = value.as_object_mut().unwrap();
            object.insert(
                "zz_future".to_owned(),
                serde_json::json!({"a": [1, 2, null]}),
            );
            object.insert("another".to_owned(), serde_json::json!("x"));
            let bytes = serde_json::to_vec(&value).unwrap();
            assert_eq!(
                Payload::from_json(kind, &bytes).unwrap(),
                payload,
                "{kind:?}"
            );
        }
        // Missing and mistyped known fields stay errors next to an unknown one.
        assert!(Payload::from_json(Kind::Takeback, br#"{"zz":1}"#).is_err());
        assert!(Payload::from_json(Kind::Takeback, br#"{"to_ply":"1","zz":1}"#).is_err());
    }

    /// FIND-B-11 B: the writer never emits a field outside the SDD table, so tolerance on read
    /// is not a licence to write.
    #[test]
    fn the_writer_never_emits_an_unknown_field() {
        for payload in one_of_each() {
            let value: serde_json::Value =
                serde_json::from_slice(&payload.to_json().unwrap()).unwrap();
            let keys: Vec<&str> = value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            for key in keys {
                assert!(
                    payload.kind().fields().contains(&key),
                    "{:?} {key}",
                    payload.kind()
                );
            }
        }
    }

    fn move_with(think: u64, white: Option<u64>, black: Option<u64>) -> Payload {
        Payload::Move(MovePayload {
            ply: 0,
            uci: "e2e4".to_owned(),
            think_ms: think,
            white_ms: white,
            black_ms: black,
            san: "e4".to_owned(),
            fen_after: "x".to_owned(),
        })
    }

    fn game_end_with(white: u64, black: u64, total: u64) -> Payload {
        Payload::GameEnd(GameEndPayload {
            result: "1-0".to_owned(),
            reason: EndReason::Checkmate,
            ended_at: 1,
            eco: None,
            opening_name: None,
            analysis_used: false,
            content_hash: "h".to_owned(),
            white_time_used_ms: white,
            black_time_used_ms: black,
            total_active_ms: total,
        })
    }

    /// CH-4 (Brian, 2026-11-22): every `u64` field is refused above `i64::MAX` at encode and at
    /// decode, with a typed error, and `i64::MAX` itself is accepted.
    #[test]
    fn ch4_u64_fields_above_i64_max_are_refused_at_encode_and_at_decode() {
        const MAX: u64 = i64::MAX as u64;
        let over = MAX + 1;
        let cases: Vec<(&str, Payload, Payload)> = vec![
            (
                "think_ms",
                move_with(MAX, None, None),
                move_with(over, None, None),
            ),
            (
                "move.white_ms",
                move_with(0, Some(MAX), Some(0)),
                move_with(0, Some(over), Some(0)),
            ),
            (
                "move.black_ms",
                move_with(0, Some(0), Some(MAX)),
                move_with(0, Some(0), Some(over)),
            ),
            (
                "clock.white_ms",
                Payload::Clock(ClockPayload {
                    white_ms: Some(MAX),
                    black_ms: Some(0),
                    running_side: None,
                }),
                Payload::Clock(ClockPayload {
                    white_ms: Some(over),
                    black_ms: Some(0),
                    running_side: None,
                }),
            ),
            (
                "clock.black_ms",
                Payload::Clock(ClockPayload {
                    white_ms: Some(0),
                    black_ms: Some(MAX),
                    running_side: None,
                }),
                Payload::Clock(ClockPayload {
                    white_ms: Some(0),
                    black_ms: Some(over),
                    running_side: None,
                }),
            ),
            (
                "white_time_used_ms",
                game_end_with(MAX, 0, 0),
                game_end_with(over, 0, 0),
            ),
            (
                "black_time_used_ms",
                game_end_with(0, MAX, 0),
                game_end_with(0, over, 0),
            ),
            (
                "total_active_ms",
                game_end_with(0, 0, MAX),
                game_end_with(0, 0, over),
            ),
        ];
        for (name, fine, too_big) in cases {
            let kind = fine.kind();
            let json = fine
                .to_json()
                .unwrap_or_else(|e| panic!("{name}: max refused: {e}"));
            assert_eq!(Payload::from_json(kind, &json).unwrap(), fine, "{name}");
            // Encode refuses.
            assert_eq!(
                too_big.to_json().unwrap_err(),
                FormatError::BadPayload,
                "{name} encode"
            );
            // Decode refuses a record another writer produced: build the JSON by raw edit.
            let text = String::from_utf8(json).unwrap();
            let raw = text.replace(&MAX.to_string(), &over.to_string());
            assert_ne!(text, raw, "{name}: test edit applied");
            assert_eq!(
                Payload::from_json(kind, raw.as_bytes()).unwrap_err(),
                FormatError::BadPayload,
                "{name} decode"
            );
            let big = JournalRecord {
                seq: 1,
                wall_ms: 0,
                payload: too_big,
            };
            assert_eq!(
                encode_record(&big).unwrap_err(),
                FormatError::BadPayload,
                "{name} record"
            );
        }
    }

    /// CH-4 (spec 3.1 (a)): the boundary is the literal `9223372036854775807`, not a constant of
    /// this crate, so a changed constant cannot hide itself.
    #[test]
    fn ch4_the_boundary_is_the_literal_i64_max() {
        let move_json = |think: u64| {
            format!(
                r#"{{"ply":0,"uci":"a1a2","think_ms":{think},"white_ms":null,"black_ms":null,"san":"x","fen_after":"f"}}"#
            )
        };
        let at = |v: u64| Payload::from_json(Kind::Move, move_json(v).as_bytes());
        assert!(at(9_223_372_036_854_775_807).is_ok());
        assert!(at(9_223_372_036_854_775_808).is_err());
        assert!(at(18_446_744_073_709_551_615).is_err());
        assert!(
            move_with(9_223_372_036_854_775_807, None, None)
                .to_json()
                .is_ok()
        );
        assert!(
            move_with(9_223_372_036_854_775_808, None, None)
                .to_json()
                .is_err()
        );
        assert_eq!(MAX_COUNT, 9_223_372_036_854_775_807);
    }

    /// CH-4: `seq` is stored as a signed integer too: a record with `seq` above `i64::MAX` is
    /// refused by the encoder and by the decoder (a CRC-valid frame built by hand).
    #[test]
    fn ch4_seq_above_i64_max_is_refused_at_encode_and_at_decode() {
        let at = |seq: u64| JournalRecord {
            seq,
            wall_ms: 0,
            payload: Payload::Takeback(TakebackPayload { to_ply: 1 }),
        };
        let max = u64::try_from(i64::MAX).unwrap();
        let bytes = encode_record(&at(max)).unwrap();
        assert_eq!(decode_record(&bytes).unwrap().0.seq, max);
        assert_eq!(
            encode_record(&at(max + 1)).unwrap_err(),
            FormatError::BadPayload
        );
        // Hand-built frame with seq = i64::MAX + 1 and a good checksum.
        let mut body = (max + 1).to_le_bytes().to_vec();
        body.push(3);
        body.extend(0i64.to_le_bytes());
        body.extend(br#"{"to_ply":1}"#);
        let len = u32::try_from(body.len()).unwrap();
        let mut covered = len.to_le_bytes().to_vec();
        covered.extend(&body);
        let mut frame = SYNC.to_vec();
        frame.extend(len.to_le_bytes());
        frame.extend(crc32(&covered).to_le_bytes());
        frame.extend(body);
        assert_eq!(decode_record(&frame).unwrap_err(), FormatError::BadPayload);
    }

    /// A CRC-valid frame built by hand: the bytes a foreign or damaged writer could leave.
    fn hand_frame(seq: u64, kind: u8, wall_ms: i64, json: &str) -> Vec<u8> {
        let mut body = seq.to_le_bytes().to_vec();
        body.push(kind);
        body.extend(wall_ms.to_le_bytes());
        body.extend(json.as_bytes());
        let len = u32::try_from(body.len()).unwrap();
        let mut covered = len.to_le_bytes().to_vec();
        covered.extend(&body);
        let mut frame = SYNC.to_vec();
        frame.extend(len.to_le_bytes());
        frame.extend(crc32(&covered).to_le_bytes());
        frame.extend(body);
        frame
    }

    fn game_end_at(ended_at: i64) -> (Payload, u8, String) {
        let payload = Payload::GameEnd(GameEndPayload {
            result: "1-0".to_owned(),
            reason: EndReason::Checkmate,
            ended_at,
            eco: None,
            opening_name: None,
            analysis_used: false,
            content_hash: "h".to_owned(),
            white_time_used_ms: 0,
            black_time_used_ms: 0,
            total_active_ms: 0,
        });
        let json = format!(
            r#"{{"result":"1-0","reason":"checkmate","ended_at":{ended_at},"eco":null,"opening_name":null,"analysis_used":false,"content_hash":"h","white_time_used_ms":0,"black_time_used_ms":0,"total_active_ms":0}}"#
        );
        (payload, 12, json)
    }

    fn abort_at(ended_at: i64) -> (Payload, u8, String) {
        let payload = Payload::Abort(AbortPayload {
            ended_at,
            content_hash: "h".to_owned(),
        });
        let json = format!(r#"{{"ended_at":{ended_at},"content_hash":"h"}}"#);
        (payload, 13, json)
    }

    fn voided_at(ended_at: i64) -> (Payload, u8, String) {
        let payload = Payload::Voided(VoidedPayload {
            reason: VoidReason::Unreadable,
            reset_player: None,
            content_hash: "h".to_owned(),
            ended_at,
        });
        let json = format!(
            r#"{{"reason":"unreadable","reset_player":null,"content_hash":"h","ended_at":{ended_at}}}"#
        );
        (payload, 14, json)
    }

    /// D3 ruling 2026-11-24 (FIND-D3-01, F2-04): `ended_at` of `GameEnd`, `Abort` and `Voided`
    /// must be above zero at encode and at decode, like the store. 1 and `i64::MAX` pass.
    #[test]
    fn d3_ended_at_at_or_below_zero_is_refused_at_encode_and_at_decode() {
        for make in [game_end_at, abort_at, voided_at] {
            for bad in [0, -1, i64::MIN] {
                let (payload, code, json) = make(bad);
                let kind = Kind::from_code(code).unwrap();
                assert_eq!(
                    payload.to_json().unwrap_err(),
                    FormatError::BadPayload,
                    "{kind:?} {bad}"
                );
                let record = JournalRecord {
                    seq: 1,
                    wall_ms: 5,
                    payload,
                };
                assert_eq!(
                    encode_record(&record).unwrap_err(),
                    FormatError::BadPayload,
                    "{kind:?} {bad}"
                );
                assert_eq!(
                    Payload::from_json(kind, json.as_bytes()).unwrap_err(),
                    FormatError::BadPayload,
                    "{kind:?} {bad}"
                );
                assert_eq!(
                    decode_record(&hand_frame(1, code, 5, &json)).unwrap_err(),
                    FormatError::BadPayload,
                    "{kind:?} {bad}"
                );
            }
            for good in [1, i64::MAX] {
                let (payload, code, json) = make(good);
                let kind = Kind::from_code(code).unwrap();
                assert_eq!(Payload::from_json(kind, json.as_bytes()).unwrap(), payload);
                let record = JournalRecord {
                    seq: 1,
                    wall_ms: 5,
                    payload,
                };
                let bytes = encode_record(&record).unwrap();
                assert_eq!(decode_record(&bytes).unwrap().0, record);
            }
        }
    }

    /// FIND-D3R-07: the `Start` record's `wall_ms` becomes `GAME.started_at`; at or below zero it
    /// is refused at encode and at decode. Other kinds keep accepting any `wall_ms`.
    #[test]
    fn d3_start_wall_ms_at_or_below_zero_is_refused_at_encode_and_at_decode() {
        let start_json = serde_json::to_string(&start()).unwrap();
        for bad in [0, -3, i64::MIN] {
            let record = JournalRecord {
                seq: 1,
                wall_ms: bad,
                payload: Payload::Start(start()),
            };
            assert_eq!(
                encode_record(&record).unwrap_err(),
                FormatError::BadPayload,
                "{bad}"
            );
            assert_eq!(
                decode_record(&hand_frame(1, 1, bad, &start_json)).unwrap_err(),
                FormatError::BadPayload,
                "{bad}"
            );
            let other = JournalRecord {
                seq: 1,
                wall_ms: bad,
                payload: Payload::Pause,
            };
            let bytes = encode_record(&other).unwrap();
            assert_eq!(decode_record(&bytes).unwrap().0, other, "{bad}");
        }
        for good in [1, i64::MAX] {
            let record = JournalRecord {
                seq: 1,
                wall_ms: good,
                payload: Payload::Start(start()),
            };
            let bytes = encode_record(&record).unwrap();
            assert_eq!(decode_record(&bytes).unwrap().0, record);
        }
    }

    /// D3-06 ruling: `matchup` is `hh`, `hc` or `cc` and equals the code of the two kinds, at
    /// encode and at decode. The long names and any other text are refused.
    #[test]
    fn d3_matchup_text_must_be_the_code_of_the_two_seat_kinds() {
        use SeatKind::{Computer as C, Human as H};
        let code = |w, b| match (w, b) {
            (H, H) => "hh",
            (C, C) => "cc",
            _ => "hc",
        };
        for (w, b) in [(H, H), (H, C), (C, H), (C, C)] {
            for text in [
                "hh",
                "hc",
                "cc",
                "ch",
                "human-human",
                "human-computer",
                "HH",
                "",
            ] {
                let mut p = start();
                (p.white_kind, p.black_kind, p.matchup) = (w, b, text.to_owned());
                let record = JournalRecord {
                    seq: 1,
                    wall_ms: 5,
                    payload: Payload::Start(p.clone()),
                };
                let json = serde_json::to_string(&p).unwrap();
                let decoded = Payload::from_json(Kind::Start, json.as_bytes());
                let encoded = encode_record(&record);
                if text == code(w, b) {
                    assert_eq!(decoded.unwrap(), Payload::Start(p));
                    assert!(encoded.is_ok());
                } else {
                    assert_eq!(
                        decoded.unwrap_err(),
                        FormatError::BadPayload,
                        "{w:?} {b:?} {text}"
                    );
                    assert_eq!(
                        encoded.unwrap_err(),
                        FormatError::BadPayload,
                        "{w:?} {b:?} {text}"
                    );
                }
            }
        }
    }

    proptest::proptest! {
        /// D3 ruling: the reader never sees a record this build's writer produced and refuses,
        /// for the new value bounds too (`ended_at`, `Start.wall_ms`, `matchup`). Also the other
        /// way: a record that breaks a bound is never encoded.
        #[test]
        fn d3_the_writer_never_writes_what_the_reader_refuses_for_times_and_matchup(
            ended_at in proptest::prelude::any::<i64>(),
            wall_ms in proptest::prelude::any::<i64>(),
            small in proptest::prelude::any::<bool>(),
            which in 0usize..4,
            matchup in 0usize..5,
            kinds in 0usize..4,
        ) {
            let squash = |v: i64| if small { v % 3 } else { v };
            let (ended_at, wall_ms) = (squash(ended_at), squash(wall_ms));
            let mut p = start();
            (p.white_kind, p.black_kind) = [
                (SeatKind::Human, SeatKind::Human),
                (SeatKind::Human, SeatKind::Computer),
                (SeatKind::Computer, SeatKind::Human),
                (SeatKind::Computer, SeatKind::Computer),
            ][kinds];
            p.matchup = ["hh", "hc", "cc", "human-human", ""][matchup].to_owned();
            let payload = match which {
                0 => game_end_at(ended_at).0,
                1 => abort_at(ended_at).0,
                2 => voided_at(ended_at).0,
                _ => Payload::Start(p.clone()),
            };
            let expected_ok = match which {
                3 => wall_ms > 0 && p.matchup == ["hh", "hc", "hc", "cc"][kinds],
                _ => ended_at > 0,
            };
            let record = JournalRecord { seq: 1, wall_ms, payload };
            match encode_record(&record) {
                Ok(bytes) => {
                    proptest::prop_assert!(expected_ok);
                    let (back, used) = decode_record(&bytes).expect("reader refuses a writer record");
                    proptest::prop_assert_eq!(back, record);
                    proptest::prop_assert_eq!(used, bytes.len());
                }
                Err(_) => proptest::prop_assert!(!expected_ok),
            }
        }
    }

    proptest::proptest! {
        /// CH-4: the reader never sees a record this build's writer produced and refuses:
        /// whatever the numbers, `encode` succeeding implies `decode` succeeding with the same
        /// value.
        #[test]
        fn the_writer_never_writes_what_the_reader_refuses(
            think in proptest::prelude::any::<u64>(),
            white in proptest::option::of(proptest::prelude::any::<u64>()),
            black in proptest::option::of(proptest::prelude::any::<u64>()),
            total in proptest::prelude::any::<u64>(),
            seq in proptest::prelude::any::<u64>(),
            small in proptest::prelude::any::<bool>(),
        ) {
            let squash = |v: u64| if small { v % 1_000 } else { v };
            let payloads = [
                move_with(squash(think), white.map(squash), black.map(squash)),
                game_end_with(squash(think), squash(total), squash(total)),
                Payload::Clock(ClockPayload {
                    white_ms: white.map(squash), black_ms: black.map(squash), running_side: None,
                }),
            ];
            for payload in payloads {
                let record = JournalRecord { seq, wall_ms: 5, payload };
                if let Ok(bytes) = encode_record(&record) {
                    let (back, used) = decode_record(&bytes).expect("reader refuses a writer record");
                    proptest::prop_assert_eq!(back, record);
                    proptest::prop_assert_eq!(used, bytes.len());
                }
            }
        }
    }

    proptest::proptest! {
        /// DD-ST-015: decode errors are typed, never panics, for arbitrary bytes.
        #[test]
        fn arbitrary_bytes_never_panic_the_decoders(
            bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..300),
            kind in 0u8..16,
        ) {
            let _ = decode_header(&bytes);
            let _ = decode_record(&bytes);
            if let Some(kind) = Kind::from_code(kind) {
                let _ = Payload::from_json(kind, &bytes);
            }
            let mut pos = 0;
            while let Ok(frame) = read_frame(&bytes, pos) {
                pos += frame.total;
            }
        }
    }
}
