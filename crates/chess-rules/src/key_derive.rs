//! The EPD derivation of a position, written once (SDD 3.11.9, D-75). `build.rs` (through
//! `build/eco_gen.rs`, with `shakmaty` values) and `PositionKey::to_epd` (with this crate's own
//! values) both call it, so the key the generator stores and the key `eco::lookup` computes
//! cannot drift apart. It takes plain values and imports nothing.

/// The first four FEN fields: placement, side to move, castling rights, en passant target.
///
/// `piece_at(i)` gives, for the square index `i` (a1 = 0, h1 = 7, a8 = 56), `(is_white, letter)`
/// with `letter` one of `p n b r q k` in lower case. `castling` is white king side, white queen
/// side, black king side, black queen side. `en_passant` is a square index, present only when a
/// legal en passant capture exists.
pub(crate) fn epd(
    piece_at: impl Fn(u8) -> Option<(bool, char)>,
    white_to_move: bool,
    castling: [bool; 4],
    en_passant: Option<u8>,
) -> String {
    let mut out = String::new();
    for rank in (0..8u8).rev() {
        let mut empty = 0u8;
        for file in 0..8u8 {
            match piece_at(rank * 8 + file) {
                None => empty += 1,
                Some((white, letter)) => {
                    if empty > 0 {
                        out.push_str(&empty.to_string());
                        empty = 0;
                    }
                    out.push(if white {
                        letter.to_ascii_uppercase()
                    } else {
                        letter
                    });
                }
            }
        }
        if empty > 0 {
            out.push_str(&empty.to_string());
        }
        if rank > 0 {
            out.push('/');
        }
    }
    out.push(' ');
    out.push(if white_to_move { 'w' } else { 'b' });
    out.push(' ');
    let rights: String = castling
        .iter()
        .zip(['K', 'Q', 'k', 'q'])
        .filter(|(has, _)| **has)
        .map(|(_, letter)| letter)
        .collect();
    out.push_str(if rights.is_empty() { "-" } else { &rights });
    out.push(' ');
    match en_passant {
        Some(index) => {
            out.push(char::from(b'a' + index % 8));
            out.push(char::from(b'1' + index / 8));
        }
        None => out.push('-'),
    }
    out
}
