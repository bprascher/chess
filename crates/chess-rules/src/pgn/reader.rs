//! DD-RUL-011 and DD-RUL-012: the PGN reader, an iterative tokenizer with an explicit variation
//! stack (no recursion over input depth), per-game error recovery and per-game text decoding
//! (SDD 3.9.2, 3.10, 3.11.8). Every loop consumes at least one byte per turn, so every input
//! terminates.

use core::fmt;

use crate::error::{MoveParseError, PgnErrorKind, PgnParseError, SyntaxReason, TagReason};
use crate::fen::Fen;
use crate::notation::{ParseMode, match_move};
use crate::position::Position;

use super::{
    ImportWarning, MAX_IMPORT_WARNINGS_PER_GAME, PgnDate, PgnGame, PgnLimits, PgnNode, PgnResult,
    PgnTags, Termination, TimeControl,
};

// ---------------------------------------------------------------- lexer

#[derive(Debug, Clone, Copy)]
struct Cursor {
    pos: usize,
    /// 1-based.
    line: usize,
    /// Offset of the first byte of the current line.
    line_start: usize,
}

struct Lexer<'a> {
    data: &'a [u8],
    c: Cursor,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

impl<'a> Lexer<'a> {
    fn new(data: &'a [u8], start: usize) -> Lexer<'a> {
        Lexer {
            data,
            c: Cursor {
                pos: start,
                line: 1,
                line_start: start,
            },
        }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.c.pos).copied()
    }

    /// Consumes one byte. `\r\n` and a lone `\r` count as one line break each (SDD 3.11.8).
    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.c.pos += 1;
        if b == b'\n' {
            self.c.line += 1;
            self.c.line_start = self.c.pos;
        } else if b == b'\r' {
            if self.peek() == Some(b'\n') {
                self.c.pos += 1;
            }
            self.c.line += 1;
            self.c.line_start = self.c.pos;
        }
        Some(b)
    }

    fn line(&self) -> usize {
        self.c.line
    }

    /// Bytes from 1.
    fn col(&self) -> usize {
        self.c.pos - self.c.line_start + 1
    }

    fn at_line_start(&self) -> bool {
        self.data
            .get(self.c.line_start..self.c.pos)
            .is_some_and(|s| s.iter().all(|b| matches!(b, b' ' | b'\t')))
    }

    /// At a DOS end-of-file mark (Ctrl-Z, 0x1A) that only whitespace and further marks follow.
    /// Old DOS tools append it to text files; the reader treats it as the end of input (RF-02).
    /// A 0x1A anywhere else is an ordinary stray byte.
    fn at_eof_mark(&self) -> bool {
        self.peek() == Some(0x1a)
            && self
                .data
                .get(self.c.pos..)
                .is_some_and(|rest| rest.iter().all(|b| *b == 0x1a || is_space(*b)))
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(is_space) {
            self.bump();
        }
    }

    fn skip_line(&mut self) {
        while let Some(b) = self.peek() {
            self.bump();
            if b == b'\n' || b == b'\r' {
                break;
            }
        }
    }

    /// Bytes up to the end of the line (the break is consumed, not returned).
    fn read_line(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(b) = self.bump() {
            if b == b'\n' || b == b'\r' {
                break;
            }
            out.push(b);
        }
        out
    }

    /// After the opening brace: the comment bytes (line breaks as spaces), or `None` when the
    /// input ends first.
    fn read_brace_comment(&mut self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            match self.bump()? {
                b'}' => return Some(out),
                b'\n' | b'\r' => out.push(b' '),
                b => out.push(b),
            }
        }
    }
}

// ---------------------------------------------------------------- failures and decoding

struct Fail {
    line: usize,
    col: usize,
    kind: PgnErrorKind,
    /// Variation depth where the error happened (the skip continues from it).
    depth: usize,
    /// The error happened in the movetext (a tag section at a line start then ends the skip).
    in_movetext: bool,
    /// The input ends (unterminated comment).
    fatal: bool,
}

impl Fail {
    fn at(lex: &Lexer<'_>, kind: PgnErrorKind) -> Fail {
        Fail::pos(lex.line(), lex.col(), kind)
    }

    fn pos(line: usize, col: usize, kind: PgnErrorKind) -> Fail {
        Fail {
            line,
            col,
            kind,
            depth: 0,
            in_movetext: false,
            fatal: false,
        }
    }
}

/// Text decoding of one game: UTF-8, or Latin-1 for the whole game when any payload was not
/// valid UTF-8 (IF-2). The first pass runs in UTF-8 mode and records the verdict.
struct Decoder {
    latin1: bool,
    all_utf8: bool,
    /// Import warnings of the game being read (B6). Dropped with the decoder when the first,
    /// UTF-8 pass is repeated as Latin-1.
    warnings: WarnLog,
}

/// The bounded warning list of one game (RC-03): at most [`MAX_IMPORT_WARNINGS_PER_GAME`]
/// warnings are kept; later ones are only counted, and the count becomes one final
/// `ImportWarning::WarningsTruncated` entry. Memory is therefore bounded by the cap, not by
/// the input size.
#[derive(Debug, Clone, Default)]
struct WarnLog {
    items: Vec<ImportWarning>,
    dropped: usize,
}

impl WarnLog {
    fn push(&mut self, w: ImportWarning) {
        if self.items.len() < MAX_IMPORT_WARNINGS_PER_GAME {
            self.items.push(w);
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    fn into_vec(self) -> Vec<ImportWarning> {
        let mut items = self.items;
        if self.dropped > 0 {
            items.push(ImportWarning::WarningsTruncated {
                dropped: self.dropped,
            });
        }
        items
    }
}

impl Decoder {
    fn decode(&mut self, bytes: Vec<u8>) -> String {
        if self.latin1 {
            return bytes.iter().map(|b| char::from(*b)).collect();
        }
        match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => {
                self.all_utf8 = false;
                String::from_utf8_lossy(e.as_bytes()).into_owned()
            }
        }
    }
}

// ---------------------------------------------------------------- movetext

struct Frame {
    nodes: Vec<PgnNode>,
    /// The position before the last node of this frame (where a variation of it starts).
    prev: Option<Position>,
    cur: Position,
}

enum End {
    Result(PgnResult),
    /// A tag section starts at a line start: the game had no terminator.
    NextGame,
    Eof,
}

fn is_delimiter(b: u8) -> bool {
    is_space(b) || matches!(b, b'{' | b'}' | b'(' | b')' | b';' | b'$')
}

fn syntax(reason: SyntaxReason) -> PgnErrorKind {
    PgnErrorKind::BadSyntax { reason }
}

fn kind_from(error: MoveParseError) -> PgnErrorKind {
    match error {
        MoveParseError::NoMatch { reason } => PgnErrorKind::IllegalSan { reason },
        MoveParseError::PromotionRequired { .. } => PgnErrorKind::IllegalSan {
            reason: crate::error::IllegalReason::PromotionRequired,
        },
        MoveParseError::Ambiguous { candidates } => PgnErrorKind::AmbiguousSan { candidates },
        MoveParseError::LowercasePiece { .. } => syntax(SyntaxReason::LowercasePieceLetter),
        MoveParseError::Empty
        | MoveParseError::TooLong
        | MoveParseError::Syntax
        | MoveParseError::BadUci => syntax(SyntaxReason::NotAMove),
    }
}

/// Attaches a comment to the last move of the current sequence. Returns `true` when the comment
/// has text and no move to attach to (dropped; reported as a warning, B6).
fn attach_comment(frames: &mut [Frame], text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    let Some(node) = frames.last_mut().and_then(|f| f.nodes.last_mut()) else {
        return true;
    };
    match &mut node.comment {
        Some(existing) => {
            existing.push(' ');
            existing.push_str(text);
        }
        None => node.comment = Some(text.to_string()),
    }
    false
}

fn is_result_word(word: &[u8]) -> bool {
    matches!(word, b"1-0" | b"0-1" | b"1/2-1/2" | b"*")
}

/// Reads the movetext from the current position to the terminator. With `allow_next_game`, a
/// tag section at a line start at depth 0 ends the game without a terminator.
fn movetext(
    lex: &mut Lexer<'_>,
    start: &Position,
    limits: &PgnLimits,
    decoder: &mut Decoder,
    allow_next_game: bool,
) -> Result<(Vec<PgnNode>, End), Fail> {
    let mut frames = vec![Frame {
        nodes: Vec::new(),
        prev: None,
        cur: start.clone(),
    }];
    // `true` right after a capture move (comments and NAGs keep it): an `e.p.` annotation may
    // follow (C-R6, desk review R8).
    let mut after_capture = false;
    let end = loop {
        lex.skip_ws();
        let Some(b) = lex.peek() else {
            break End::Eof;
        };
        if lex.at_eof_mark() {
            break End::Eof;
        }
        let (line, col) = (lex.line(), lex.col());
        let fail = |frames: &[Frame], kind| Fail {
            line,
            col,
            kind,
            depth: frames.len().saturating_sub(1),
            in_movetext: true,
            fatal: false,
        };
        match b {
            b'{' => {
                lex.bump();
                let Some(bytes) = lex.read_brace_comment() else {
                    return Err(Fail {
                        fatal: true,
                        ..fail(&frames, PgnErrorKind::UnterminatedComment)
                    });
                };
                let text = decoder.decode(bytes);
                if attach_comment(&mut frames, &text) {
                    decoder
                        .warnings
                        .push(ImportWarning::CommentDropped { line, col });
                }
            }
            b';' => {
                lex.bump();
                let text = decoder.decode(lex.read_line());
                if attach_comment(&mut frames, &text) {
                    decoder
                        .warnings
                        .push(ImportWarning::CommentDropped { line, col });
                }
            }
            b'%' if lex.at_line_start() => lex.skip_line(),
            b'[' if allow_next_game && frames.len() == 1 && lex.at_line_start() => {
                break End::NextGame;
            }
            b'}' => {
                lex.bump();
                return Err(fail(&frames, syntax(SyntaxReason::StrayClosingBrace)));
            }
            b'(' => {
                lex.bump();
                after_capture = false;
                let depth_limit = limits
                    .max_variation_depth
                    .min(super::MAX_VARIATION_DEPTH_CEILING);
                if frames.len() > depth_limit {
                    return Err(fail(
                        &frames,
                        PgnErrorKind::VariationTooDeep { limit: depth_limit },
                    ));
                }
                let Some(origin) = frames
                    .last()
                    .filter(|f| !f.nodes.is_empty())
                    .and_then(|f| f.prev.clone())
                else {
                    return Err(fail(&frames, syntax(SyntaxReason::VariationWithoutMove)));
                };
                frames.push(Frame {
                    nodes: Vec::new(),
                    prev: None,
                    cur: origin,
                });
            }
            b')' => {
                lex.bump();
                after_capture = false;
                if frames.len() == 1 {
                    return Err(fail(&frames, PgnErrorKind::UnbalancedVariation));
                }
                if let Some(done) = frames.pop()
                    && !done.nodes.is_empty()
                    && let Some(last) = frames.last_mut().and_then(|f| f.nodes.last_mut())
                {
                    last.variations.push(done.nodes);
                }
            }
            b'$' => {
                lex.bump();
                let mut digits = Vec::new();
                while let Some(d) = lex.peek().filter(u8::is_ascii_digit) {
                    digits.push(d);
                    lex.bump();
                }
                let nag = std::str::from_utf8(&digits)
                    .ok()
                    .filter(|d| d.len() <= 3)
                    .and_then(|d| d.parse::<u8>().ok());
                let Some(nag) = nag else {
                    return Err(fail(&frames, syntax(SyntaxReason::BadNag)));
                };
                match frames.last_mut().and_then(|f| f.nodes.last_mut()) {
                    Some(node) => node.nags.push(nag),
                    None => return Err(fail(&frames, syntax(SyntaxReason::NagWithoutMove))),
                }
            }
            _ => {
                let mut word = Vec::new();
                while let Some(c) = lex.peek().filter(|c| !is_delimiter(*c)) {
                    word.push(c);
                    lex.bump();
                }
                if is_result_word(&word) {
                    if frames.len() == 1 {
                        let token = std::str::from_utf8(&word).unwrap_or("*");
                        break End::Result(
                            PgnResult::from_token(token).unwrap_or(PgnResult::Unknown),
                        );
                    }
                    continue;
                }
                if is_en_passant_note(&word) && core::mem::take(&mut after_capture) {
                    continue;
                }
                after_capture = false;
                let (word_core, glued_ep) = strip_glued_en_passant(&word);
                let san_bytes = match move_number_rest(word_core) {
                    Some([]) => continue,
                    Some(rest) => rest,
                    None => word_core,
                };
                let Ok(text) = std::str::from_utf8(san_bytes) else {
                    return Err(fail(&frames, syntax(SyntaxReason::NotText)));
                };
                let Some(top) = frames.last_mut() else {
                    return Err(fail(&frames, syntax(SyntaxReason::Internal)));
                };
                let (matched, nag) = match_move(&top.cur, text, ParseMode::Pgn)
                    .map_err(|e| fail(&frames, kind_from(e)))?;
                let Some(top) = frames.last_mut() else {
                    return Err(fail(&frames, syntax(SyntaxReason::Internal)));
                };
                let next = top
                    .cur
                    .play(matched.mv)
                    .map_err(|e| fail(&frames, PgnErrorKind::IllegalSan { reason: e.reason }))?;
                let is_capture = matched.san.as_str().contains('x');
                if glued_ep && !is_capture {
                    return Err(fail(&frames, syntax(SyntaxReason::NotAMove)));
                }
                after_capture = is_capture;
                let Some(top) = frames.last_mut() else {
                    return Err(fail(&frames, syntax(SyntaxReason::Internal)));
                };
                top.prev = Some(core::mem::replace(&mut top.cur, next));
                let mut node = PgnNode::new(matched.mv);
                node.nags.extend(nag);
                top.nodes.push(node);
            }
        }
    };
    if frames.len() > 1 {
        return Err(Fail {
            line: lex.line(),
            col: lex.col(),
            kind: PgnErrorKind::UnbalancedVariation,
            depth: frames.len() - 1,
            in_movetext: true,
            fatal: false,
        });
    }
    let nodes = frames
        .into_iter()
        .next()
        .map(|f| f.nodes)
        .unwrap_or_default();
    Ok((nodes, end))
}

/// `e.p.` or `e.p` (any ASCII case) as a word of its own: the en passant annotation.
fn is_en_passant_note(word: &[u8]) -> bool {
    word.eq_ignore_ascii_case(b"e.p.") || word.eq_ignore_ascii_case(b"e.p")
}

/// A word such as `exd6e.p.` split into the move and a flag for the glued annotation. The flag
/// is checked against the move once it is matched (the move must be a capture).
fn strip_glued_en_passant(word: &[u8]) -> (&[u8], bool) {
    for suffix in [&b"e.p."[..], &b"e.p"[..]] {
        if word.len() > suffix.len()
            && let Some(split) = word.len().checked_sub(suffix.len())
            && let (Some(head), Some(tail)) = (word.get(..split), word.get(split..))
            && tail.eq_ignore_ascii_case(suffix)
        {
            return (head, true);
        }
    }
    (word, false)
}

/// `Some(rest)` when the word is a move number (digits then dots, or digits alone, or only
/// dots) with `rest` glued behind it; `None` when it is not (a SAN word, such as `0-0`).
fn move_number_rest(word: &[u8]) -> Option<&[u8]> {
    let digits = word.iter().take_while(|b| b.is_ascii_digit()).count();
    let after = word.get(digits..)?;
    if digits == 0 {
        return if after.first() == Some(&b'.') {
            Some(after.get(after.iter().take_while(|b| **b == b'.').count()..)?)
        } else {
            None
        };
    }
    if after.is_empty() {
        return Some(after);
    }
    if after.first() == Some(&b'.') {
        return after.get(after.iter().take_while(|b| **b == b'.').count()..);
    }
    None
}

/// Movetext of a saved line from `start` (DD-RUL-012). A result token ends the text. Errors
/// carry `game_index` 1.
pub fn parse_line(start: &Position, text: &str) -> Result<Vec<PgnNode>, PgnParseError> {
    let mut lex = Lexer::new(text.as_bytes(), 0);
    let mut decoder = Decoder {
        latin1: false,
        all_utf8: true,
        warnings: WarnLog::default(),
    };
    let (nodes, _end) = movetext(&mut lex, start, &PgnLimits::default(), &mut decoder, false)
        .map_err(|f| PgnParseError {
            game_index: 1,
            line: f.line,
            col: f.col,
            kind: f.kind,
        })?;
    Ok(nodes)
}

// ---------------------------------------------------------------- skipping (error recovery)

/// Skips to just after the next result token at depth 0, outside comments and quoted strings.
/// With `in_movetext`, a tag section at a line start at depth 0 also ends the skip (the game
/// had no terminator).
fn skip_to_terminator(lex: &mut Lexer<'_>, mut depth: usize, in_movetext: bool) {
    loop {
        lex.skip_ws();
        let Some(b) = lex.peek() else { return };
        match b {
            b'{' => {
                lex.bump();
                if lex.read_brace_comment().is_none() {
                    return;
                }
            }
            b';' => lex.skip_line(),
            b'%' if lex.at_line_start() => lex.skip_line(),
            b'"' => {
                lex.bump();
                while let Some(c) = lex.peek() {
                    if c == b'\n' || c == b'\r' {
                        break;
                    }
                    lex.bump();
                    if c == b'"' {
                        break;
                    }
                }
            }
            b'(' => {
                lex.bump();
                depth = depth.saturating_add(1);
            }
            b')' => {
                lex.bump();
                depth = depth.saturating_sub(1);
            }
            b'}' => {
                lex.bump();
            }
            b'[' if in_movetext && depth == 0 && lex.at_line_start() => return,
            _ => {
                let mut word = Vec::new();
                while let Some(c) = lex.peek().filter(|c| !is_delimiter(*c) && *c != b'"') {
                    word.push(c);
                    lex.bump();
                }
                if word.is_empty() {
                    // `$` or a stray byte that no arm took: step over it.
                    lex.bump();
                } else if depth == 0 && is_result_word(&word) {
                    return;
                }
            }
        }
    }
}

// ---------------------------------------------------------------- reader

/// Iterator over the games of a PGN byte string (IFP-RUL-5). One bad game is one `Err` item and
/// never ends the iteration, except an unterminated comment, which ends the input and is
/// reported once. The caller enforces the file-size limit (TBD-25).
pub struct PgnReader<'a> {
    lex: Lexer<'a>,
    limits: PgnLimits,
    game_index: usize,
    done: bool,
    /// Comments with text found between games (before the tag section, or before the movetext
    /// of a game without tags). They belong to no move; they are reported on the game that
    /// follows (B6) and dropped if no game follows.
    pending: WarnLog,
}

impl fmt::Debug for PgnReader<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgnReader")
            .field("offset", &self.lex.c.pos)
            .field("game_index", &self.game_index)
            .field("done", &self.done)
            .finish()
    }
}

impl<'a> PgnReader<'a> {
    /// A leading UTF-8 byte-order mark is dropped.
    pub fn new(bytes: &'a [u8], limits: PgnLimits) -> PgnReader<'a> {
        let start = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
            3
        } else {
            0
        };
        PgnReader {
            lex: Lexer::new(bytes, start),
            limits,
            game_index: 0,
            done: false,
            pending: WarnLog::default(),
        }
    }

    /// Skips blanks, `%` lines and comments between games; `Ok(false)` at the end of input.
    fn skip_between_games(&mut self) -> Result<bool, Fail> {
        loop {
            self.lex.skip_ws();
            match self.lex.peek() {
                None => return Ok(false),
                Some(b'%') if self.lex.at_line_start() => self.lex.skip_line(),
                Some(b';') => {
                    let (line, col) = (self.lex.line(), self.lex.col());
                    self.lex.bump();
                    let bytes = self.lex.read_line();
                    self.note_comment(&bytes, line, col);
                }
                Some(b'{') => {
                    let (line, col) = (self.lex.line(), self.lex.col());
                    self.lex.bump();
                    let Some(bytes) = self.lex.read_brace_comment() else {
                        return Err(Fail {
                            fatal: true,
                            ..Fail::pos(line, col, PgnErrorKind::UnterminatedComment)
                        });
                    };
                    self.note_comment(&bytes, line, col);
                }
                Some(_) if self.lex.at_eof_mark() => return Ok(false),
                Some(b) if b != b'[' && self.at_stray_result() => {}
                Some(_) => return Ok(true),
            }
        }
    }

    /// Remembers a comment that has text and belongs to no game (reported on the next game).
    fn note_comment(&mut self, bytes: &[u8], line: usize, col: usize) {
        if !String::from_utf8_lossy(bytes).trim().is_empty() {
            self.pending
                .push(ImportWarning::CommentDropped { line, col });
        }
    }

    /// RF-02: a result word (`*`, `1-0`, `0-1`, `1/2-1/2`) where a game would start, with no
    /// tag section before it, is a redundant terminator (a file that ends in `*` after the
    /// last game's own terminator). It is consumed and is not a game. A game that has tags
    /// and no moves (`[Event "x"] *`) is unaffected: its tags are read first.
    fn at_stray_result(&mut self) -> bool {
        let mut end = self.lex.c.pos;
        while self
            .lex
            .data
            .get(end)
            .is_some_and(|b| !is_delimiter(*b) && *b != b'"')
        {
            end += 1;
        }
        let is_result = self
            .lex
            .data
            .get(self.lex.c.pos..end)
            .is_some_and(is_result_word);
        if is_result {
            while self.lex.c.pos < end {
                self.lex.bump();
            }
        }
        is_result
    }

    fn parse_tag(&mut self, tags: &mut RawTags, decoder: &mut Decoder) -> Result<(), Fail> {
        let lex = &mut self.lex;
        lex.bump(); // `[`
        let skip_blanks = |lex: &mut Lexer<'_>| {
            while lex.peek().is_some_and(|b| b == b' ' || b == b'\t') {
                lex.bump();
            }
        };
        skip_blanks(lex);
        let bad = |lex: &Lexer<'_>, reason| Fail::at(lex, PgnErrorKind::BadTag { reason });
        let (nline, ncol) = (lex.line(), lex.col());
        let mut name = Vec::new();
        while let Some(b) = lex
            .peek()
            .filter(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            name.push(b);
            lex.bump();
        }
        if !name.first().is_some_and(u8::is_ascii_uppercase) {
            return Err(Fail::pos(
                nline,
                ncol,
                PgnErrorKind::BadTag {
                    reason: TagReason::NameStart,
                },
            ));
        }
        let name = String::from_utf8_lossy(&name).into_owned();
        skip_blanks(lex);
        if lex.peek() != Some(b'"') {
            return Err(bad(lex, TagReason::ValueNotQuoted));
        }
        let (vline, vcol) = (lex.line(), lex.col());
        lex.bump();
        let mut value = Vec::new();
        loop {
            match lex.peek() {
                None | Some(b'\n' | b'\r') => {
                    return Err(Fail::pos(
                        vline,
                        vcol,
                        PgnErrorKind::BadTag {
                            reason: TagReason::ValueNotClosed,
                        },
                    ));
                }
                Some(b'"') => {
                    lex.bump();
                    break;
                }
                Some(b'\\') => {
                    lex.bump();
                    match lex.peek() {
                        Some(c @ (b'"' | b'\\')) => {
                            value.push(c);
                            lex.bump();
                        }
                        _ => value.push(b'\\'),
                    }
                }
                Some(c) => {
                    value.push(c);
                    lex.bump();
                }
            }
        }
        skip_blanks(lex);
        if lex.peek() != Some(b']') {
            return Err(bad(lex, TagReason::MissingClosingBracket));
        }
        lex.bump();
        let value = decoder.decode(value);
        tags.set(&name, value, vline, vcol);
        Ok(())
    }

    fn parse_game(&mut self, latin1: bool) -> Result<(PgnGame, bool), Fail> {
        let mut decoder = Decoder {
            latin1,
            all_utf8: true,
            // Between-game comments met before this game come first (B6); the Latin-1 pass
            // starts from the same list because the first pass's list is dropped.
            warnings: self.pending.clone(),
        };
        if latin1 {
            decoder.warnings.push(ImportWarning::Latin1Fallback);
        }
        let mut raw = RawTags::default();
        let mut tags_read = false;
        loop {
            let before = self.lex.line();
            self.lex.skip_ws();
            if self.lex.peek() == Some(b'[') {
                if tags_read && self.lex.line() > before + 1 {
                    // RF-03: a blank line and then another tag section. The tags above have
                    // no movetext; they are not merged into the next game. The reader stays
                    // at this `[`, which starts the next game.
                    return Err(Fail {
                        in_movetext: true,
                        ..Fail::at(
                            &self.lex,
                            PgnErrorKind::BadSyntax {
                                reason: SyntaxReason::TagSectionWithoutMovetext,
                            },
                        )
                    });
                }
                self.parse_tag(&mut raw, &mut decoder)?;
                tags_read = true;
            } else {
                break;
            }
        }
        // RC-13: at most one warning per typed tag, describing the value that is used.
        for w in raw.tag_warnings.drain(..) {
            decoder.warnings.push(w);
        }
        let start = match raw.fen.take() {
            Some((text, line, col)) if raw.set_up.as_deref() != Some("0") => {
                let fen =
                    Fen::parse(&text).map_err(|e| Fail::pos(line, col, PgnErrorKind::BadFen(e)))?;
                Position::from_fen(&fen)
            }
            _ => Position::standard(),
        };
        let (moves, end) = movetext(&mut self.lex, &start, &self.limits, &mut decoder, true)?;
        let result = match end {
            End::Result(r) => r,
            End::NextGame | End::Eof => raw.result.unwrap_or(PgnResult::Unknown),
        };
        let warnings = decoder.warnings.into_vec();
        Ok((
            PgnGame {
                tags: raw.tags,
                start,
                moves,
                result,
                warnings,
            },
            decoder.all_utf8,
        ))
    }
}

/// Tags as read, before the game is assembled.
#[derive(Default)]
struct RawTags {
    tags: PgnTags,
    result: Option<PgnResult>,
    set_up: Option<String>,
    fen: Option<(String, usize, usize)>,
    /// The warning of the current value of each typed tag (`Date`, `TimeControl`,
    /// `Termination`); a later tag of the same name replaces the earlier warning (RC-13).
    tag_warnings: Vec<ImportWarning>,
}

impl RawTags {
    /// A repeated tag name replaces the earlier value. Typed tags whose value does not parse
    /// (`Date`, `TimeControl`, `Termination`) fall back to unknown / absent.
    fn set(&mut self, name: &str, value: String, line: usize, col: usize) {
        let t = &mut self.tags;
        match name {
            "Event" => t.event = value,
            "Site" => t.site = value,
            "Round" => t.round = value,
            "White" => t.white = value,
            "Black" => t.black = value,
            "Date" => {
                t.date = PgnDate::parse(&value).unwrap_or_default();
                self.tag_warnings
                    .retain(|w| !matches!(w, ImportWarning::DateIgnored { .. }));
                if t.date.to_string() != value {
                    self.tag_warnings.push(ImportWarning::DateIgnored { value });
                }
            }
            "Result" => self.result = PgnResult::from_token(&value),
            "TimeControl" => {
                t.time_control = TimeControl::parse(&value);
                self.tag_warnings
                    .retain(|w| !matches!(w, ImportWarning::TimeControlIgnored { .. }));
                if t.time_control.is_none() && value != "-" && value != "?" {
                    self.tag_warnings
                        .push(ImportWarning::TimeControlIgnored { value });
                }
            }
            "Termination" => {
                t.termination = Termination::parse(&value);
                self.tag_warnings
                    .retain(|w| !matches!(w, ImportWarning::TerminationIgnored { .. }));
                if t.termination.is_none() {
                    self.tag_warnings
                        .push(ImportWarning::TerminationIgnored { value });
                }
            }
            "SetUp" => self.set_up = Some(value),
            "FEN" => self.fen = Some((value, line, col)),
            _ => {
                t.extras.insert(name.to_string(), value);
            }
        }
    }
}

impl Iterator for PgnReader<'_> {
    type Item = Result<PgnGame, PgnParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let began = self.lex.c.pos;
        let outcome = match self.skip_between_games() {
            Ok(false) => {
                self.done = true;
                return None;
            }
            Ok(true) => {
                self.game_index += 1;
                let snapshot = self.lex.c;
                match self.parse_game(false) {
                    Ok((_, false)) => {
                        // A payload was not UTF-8: decode the whole game as Latin-1.
                        self.lex.c = snapshot;
                        self.parse_game(true).map(|(game, _)| game)
                    }
                    other => other.map(|(game, _)| game),
                }
            }
            Err(fail) => {
                self.game_index += 1;
                Err(fail)
            }
        };
        // The game (if any) already carries the between-game warnings met before it.
        self.pending = WarnLog::default();
        let item = match outcome {
            Ok(game) => Ok(game),
            Err(fail) => {
                if fail.fatal {
                    self.done = true;
                } else {
                    skip_to_terminator(&mut self.lex, fail.depth, fail.in_movetext);
                }
                Err(PgnParseError {
                    game_index: self.game_index,
                    line: fail.line,
                    col: fail.col,
                    kind: fail.kind,
                })
            }
        };
        if self.lex.c.pos == began {
            // Progress guard: no path above should get here.
            self.lex.bump();
        }
        Some(item)
    }
}
