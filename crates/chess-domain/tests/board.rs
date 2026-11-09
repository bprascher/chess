//! UT-DOM-080 to 084, 104, 110 (DD-DOM-017): `BoardGrid`. Written before the code.
//! UT-DOM-080 is specified as `insta` snapshots; `insta` is not among the vetted crates of
//! dependency-lookup-step1.md for this task, so the snapshots are exact expected line lists
//! written here (finding F-B1 in step1-domain-B-evidence.md).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    BoardError, BoardGrid, BoardStyle, BoardView, Cell, Highlight, Orientation, Shade, render_text,
};

const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const PLAIN: BoardView = BoardView {
    last_move: None,
    rewound: false,
};

fn view(last: Option<&str>, rewound: bool) -> BoardView {
    BoardView {
        last_move: last.map(str::to_string),
        rewound,
    }
}

fn grid(fen: &str, v: &BoardView, style: BoardStyle, o: Orientation) -> BoardGrid {
    BoardGrid::from_fen(fen, v, style, o).unwrap()
}

fn lines(fen: &str, v: &BoardView, style: BoardStyle, o: Orientation) -> Vec<String> {
    render_text(&grid(fen, v, style, o), &style)
}

fn row(label: &str, cells: [&str; 8]) -> String {
    format!("{label} {} {label}", cells.concat())
}

/// Display width: characters, with ANSI SGR sequences removed.
fn visible(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for d in chars.by_ref() {
                if d == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ------------------------------------------------------------------ UT-DOM-084, 080

#[test]
fn ut_dom_084_start_position_plain_white_at_the_bottom() {
    let got = lines(START, &PLAIN, BoardStyle::Plain, Orientation::White);
    let top = "   A  B  C  D  E  F  G  H   ".to_string();
    let want = vec![
        top.clone(),
        row(
            "8",
            [" r ", " n ", " b ", " q ", " k ", " b ", " n ", " r "],
        ),
        row("7", [" p "; 8]),
        row("6", [" . "; 8]),
        row("5", [" . "; 8]),
        row("4", [" . "; 8]),
        row("3", [" . "; 8]),
        row("2", [" P "; 8]),
        row(
            "1",
            [" R ", " N ", " B ", " Q ", " K ", " B ", " N ", " R "],
        ),
        top,
    ];
    assert_eq!(got, want);
    assert_eq!(got[1], "8  r  n  b  q  k  b  n  r  8");
}

#[test]
fn ut_dom_084_start_position_plain_black_at_the_bottom() {
    let got = lines(START, &PLAIN, BoardStyle::Plain, Orientation::Black);
    let top = "   H  G  F  E  D  C  B  A   ".to_string();
    let want = vec![
        top.clone(),
        row(
            "1",
            [" R ", " N ", " B ", " K ", " Q ", " B ", " N ", " R "],
        ),
        row("2", [" P "; 8]),
        row("3", [" . "; 8]),
        row("4", [" . "; 8]),
        row("5", [" . "; 8]),
        row("6", [" . "; 8]),
        row("7", [" p "; 8]),
        row(
            "8",
            [" r ", " n ", " b ", " k ", " q ", " b ", " n ", " r "],
        ),
        top,
    ];
    assert_eq!(got, want);
}

#[test]
fn ut_dom_080_unicode_and_nocolor_glyphs_and_shades() {
    for style in [BoardStyle::Unicode, BoardStyle::NoColor] {
        let got = lines(START, &PLAIN, style, Orientation::White);
        assert_eq!(
            got[1],
            row(
                "8",
                [
                    " \u{265c} ",
                    " \u{265e} ",
                    " \u{265d} ",
                    " \u{265b} ",
                    " \u{265a} ",
                    " \u{265d} ",
                    " \u{265e} ",
                    " \u{265c} "
                ]
            )
        );
        assert_eq!(got[7], row("2", [" \u{2659} "; 8]));
        assert_eq!(
            got[8],
            row(
                "1",
                [
                    " \u{2656} ",
                    " \u{2658} ",
                    " \u{2657} ",
                    " \u{2655} ",
                    " \u{2654} ",
                    " \u{2657} ",
                    " \u{2658} ",
                    " \u{2656} "
                ]
            )
        );
        // a6 is light, b6 dark: the dark empty squares carry a middle dot
        assert_eq!(
            got[3],
            row(
                "6",
                [
                    "   ", " \u{b7} ", "   ", " \u{b7} ", "   ", " \u{b7} ", "   ", " \u{b7} "
                ]
            )
        );
        assert_eq!(
            got[4],
            row(
                "5",
                [
                    " \u{b7} ", "   ", " \u{b7} ", "   ", " \u{b7} ", "   ", " \u{b7} ", "   "
                ]
            )
        );
    }
}

#[test]
fn ut_dom_080_last_move_check_and_rewound_highlights_in_plain() {
    // after 1.e4: e2 and e4 are bracketed
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let got = lines(
        fen,
        &view(Some("e2e4"), false),
        BoardStyle::Plain,
        Orientation::White,
    );
    assert_eq!(
        got[5],
        row(
            "4",
            [" . ", " . ", " . ", " . ", "[P]", " . ", " . ", " . "]
        )
    );
    assert_eq!(
        got[7],
        row(
            "2",
            [" P ", " P ", " P ", " P ", "[.]", " P ", " P ", " P "]
        )
    );
    // rewound: the same squares in parentheses
    let rw = lines(
        fen,
        &view(Some("e2e4"), true),
        BoardStyle::Plain,
        Orientation::White,
    );
    assert_eq!(
        rw[5],
        row(
            "4",
            [" . ", " . ", " . ", " . ", "(P)", " . ", " . ", " . "]
        )
    );
    assert_eq!(rw[7][14..17], *"(.)");
    // check: the king of the side to move in angle brackets (white king e1 attacked by a rook)
    let chk = "4k3/8/8/8/8/8/4r3/4K3 w - - 0 1";
    let c = lines(chk, &PLAIN, BoardStyle::Plain, Orientation::White);
    assert_eq!(
        c[8],
        row(
            "1",
            [" . ", " . ", " . ", " . ", "<K>", " . ", " . ", " . "]
        )
    );
    assert_eq!(
        c[1],
        row(
            "8",
            [" . ", " . ", " . ", " . ", " k ", " . ", " . ", " . "]
        )
    );
    // the king of the side NOT to move is not marked even if attacked (cannot occur legally, but
    // the flag follows the side to move)
    // orientation flips the highlight with the board
    let flipped = lines(
        fen,
        &view(Some("e2e4"), false),
        BoardStyle::Plain,
        Orientation::Black,
    );
    assert_eq!(
        flipped[4][..],
        *row(
            "4",
            [" . ", " . ", " . ", "[P]", " . ", " . ", " . ", " . "]
        )
        .as_str()
    );
}

#[test]
fn ut_dom_080_cells_carry_glyph_shade_and_highlight() {
    let fen = "4k3/8/8/8/8/8/4r3/4K3 w - - 0 1";
    let g = grid(
        fen,
        &view(Some("e2e1"), true),
        BoardStyle::Unicode,
        Orientation::White,
    );
    let e1: Cell = g.rows[7][4];
    assert_eq!(e1.glyph, '\u{2654}');
    assert_eq!(
        e1.highlight,
        Highlight {
            last_move_from: false,
            last_move_to: true,
            check: true,
            rewound: true
        }
    );
    assert_eq!(
        e1.shade,
        Shade::Dark,
        "e1: file 4 + rank 0 is even, like a1"
    );
}

#[test]
fn ut_dom_080_a1_is_dark_and_h1_is_light() {
    let g = grid(START, &PLAIN, BoardStyle::Plain, Orientation::White);
    assert_eq!(g.rows[7][0].shade, Shade::Dark, "a1");
    assert_eq!(g.rows[7][7].shade, Shade::Light, "h1");
    assert_eq!(g.rows[0][0].shade, Shade::Light, "a8");
    let b = grid(START, &PLAIN, BoardStyle::Plain, Orientation::Black);
    assert_eq!(b.rows[0][7].shade, Shade::Dark, "a1 is top right");
}

#[test]
fn ut_dom_080_ansi_colours_squares_and_highlights() {
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let l = lines(
        fen,
        &view(Some("e2e4"), false),
        BoardStyle::Ansi,
        Orientation::White,
    );
    // colour codes appear, every cell is reset, and the visible text is the plain layout with
    // no bracket marks (colour conveys the highlight)
    assert!(l[1].contains('\u{1b}'));
    for line in &l[1..9] {
        assert_eq!(line.matches("\u{1b}[0m").count(), 8, "{line:?}");
    }
    let vis: Vec<String> = l.iter().map(|s| visible(s)).collect();
    assert_eq!(
        vis[5],
        row(
            "4",
            [
                "   ",
                "   ",
                "   ",
                "   ",
                " \u{2659} ",
                "   ",
                "   ",
                "   "
            ]
        )
    );
    // the highlighted squares use another background than their neighbours
    let e4_cell = l[5].split("\u{1b}[0m").nth(4).unwrap();
    let d4_cell = l[5].split("\u{1b}[0m").nth(3).unwrap();
    assert_ne!(
        e4_cell.split('m').next(),
        d4_cell.split('m').next(),
        "the last-move square differs from a plain square"
    );
}

// ------------------------------------------------------------------ UT-DOM-081, 082, 083

#[test]
fn ut_dom_081_labels_on_all_four_sides_flip_with_the_orientation() {
    let w = grid(START, &PLAIN, BoardStyle::Plain, Orientation::White);
    let files = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
    let ranks_top_down = ['8', '7', '6', '5', '4', '3', '2', '1'];
    assert_eq!(w.top_labels, files);
    assert_eq!(w.bottom_labels, files);
    assert_eq!(w.left_labels, ranks_top_down);
    assert_eq!(w.right_labels, ranks_top_down);
    let b = grid(START, &PLAIN, BoardStyle::Plain, Orientation::Black);
    let mut rf = files;
    rf.reverse();
    let mut rr = ranks_top_down;
    rr.reverse();
    assert_eq!(b.top_labels, rf);
    assert_eq!(b.bottom_labels, rf);
    assert_eq!(b.left_labels, rr);
    assert_eq!(b.right_labels, rr);
    // and in the text
    let t = lines(START, &PLAIN, BoardStyle::Plain, Orientation::White);
    assert_eq!(t.len(), 10);
    assert_eq!(t[0], t[9]);
    assert!(t[1].starts_with('8') && t[1].ends_with('8'));
    assert!(t[8].starts_with('1') && t[8].ends_with('1'));
}

#[test]
fn ut_dom_082_ui_4_only_ansi_writes_an_escape_byte() {
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    for o in [Orientation::White, Orientation::Black] {
        for style in [BoardStyle::Plain, BoardStyle::Unicode, BoardStyle::NoColor] {
            for l in lines(fen, &view(Some("e2e4"), true), style, o) {
                assert!(!l.contains('\u{1b}'), "{style:?} {l:?}");
            }
        }
        let ansi = lines(fen, &view(Some("e2e4"), true), BoardStyle::Ansi, o);
        assert!(ansi.iter().any(|l| l.contains('\u{1b}')));
        // the label rows have no colour
        assert!(!ansi[0].contains('\u{1b}') && !ansi[9].contains('\u{1b}'));
    }
}

#[test]
fn ut_dom_083_r_7_every_rendered_square_is_three_columns_wide() {
    let fens = [
        START,
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "4k3/8/8/8/8/8/4r3/4K3 w - - 0 1",
    ];
    for fen in fens {
        for o in [Orientation::White, Orientation::Black] {
            for style in [
                BoardStyle::Plain,
                BoardStyle::Unicode,
                BoardStyle::Ansi,
                BoardStyle::NoColor,
            ] {
                let l = lines(fen, &view(Some("e2e1"), true), style, o);
                assert_eq!(l.len(), 10);
                for line in &l {
                    assert_eq!(visible(line).chars().count(), 28, "{style:?} {line:?}");
                }
                for line in &l[1..9] {
                    // after the label and a space, eight cells of three characters
                    let v: Vec<char> = visible(line).chars().collect();
                    let cells: String = v[2..26].iter().collect();
                    assert_eq!(cells.chars().count(), 24);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ UT-DOM-104

#[test]
fn ut_dom_104_invalid_fen_and_invalid_last_move_return_board_error() {
    for bad in [
        "",
        "not a fen",
        "8/8/8/8/8/8/8/8 w - - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0",
    ] {
        assert_eq!(
            BoardGrid::from_fen(bad, &PLAIN, BoardStyle::Plain, Orientation::White).err(),
            Some(BoardError::BadFen),
            "{bad:?}"
        );
    }
    for bad in ["", "e2", "e2e9", "xyz", "e2e4e6", "E2E4"] {
        assert_eq!(
            BoardGrid::from_fen(
                START,
                &view(Some(bad), false),
                BoardStyle::Plain,
                Orientation::White
            )
            .err(),
            Some(BoardError::BadLastMove),
            "{bad:?}"
        );
    }
    // a valid UCI last move sets exactly the two highlights; a promotion text is accepted
    let g = grid(
        START,
        &view(Some("e2e4"), false),
        BoardStyle::Plain,
        Orientation::White,
    );
    let flagged: Vec<(usize, usize)> = (0..8)
        .flat_map(|r| (0..8).map(move |c| (r, c)))
        .filter(|(r, c)| {
            let h = g.rows[*r][*c].highlight;
            h.last_move_from || h.last_move_to
        })
        .collect();
    assert_eq!(flagged, vec![(4, 4), (6, 4)]);
    assert!(g.rows[6][4].highlight.last_move_from);
    assert!(g.rows[4][4].highlight.last_move_to);
    assert!(
        BoardGrid::from_fen(
            START,
            &view(Some("e7e8q"), false),
            BoardStyle::Plain,
            Orientation::White
        )
        .is_ok()
    );
    // rewound without a last move marks nothing
    let g = grid(
        START,
        &view(None, true),
        BoardStyle::Plain,
        Orientation::White,
    );
    assert!(
        g.rows
            .iter()
            .flatten()
            .all(|c| c.highlight == Highlight::default())
    );
    assert!(!BoardError::BadFen.to_string().is_empty());
    assert!(!BoardError::BadLastMove.to_string().is_empty());
}

#[test]
fn ut_dom_104_no_rules_type_appears_in_the_public_signature() {
    // `from_fen` takes &str, a BoardView of strings and two domain enums, and returns
    // domain types only: this call compiles without importing chess_rules.
    let g = BoardGrid::from_fen(
        START,
        &BoardView {
            last_move: Some("g1f3".into()),
            rewound: false,
        },
        BoardStyle::Unicode,
        Orientation::White,
    )
    .unwrap();
    assert_eq!(g.rows.len(), 8);
}

// ------------------------------------------------------------------ UT-DOM-110

#[test]
fn ut_dom_110_xc_12_orientation_and_style_names() {
    assert_eq!(format!("{:?}", Orientation::White), "White");
    assert_eq!(format!("{:?}", Orientation::Black), "Black");
    let names: Vec<String> = [
        BoardStyle::Plain,
        BoardStyle::Unicode,
        BoardStyle::Ansi,
        BoardStyle::NoColor,
    ]
    .iter()
    .map(|s| format!("{s:?}"))
    .collect();
    assert_eq!(names, ["Plain", "Unicode", "Ansi", "NoColor"]);
}

/// The SGR background code of one cell of a rendered Ansi line (`line` is a rank row).
fn bg(line: &str, column: usize) -> String {
    let chunk = line.split("\u{1b}[0m").nth(column).unwrap();
    let start = chunk.find("\u{1b}[4").unwrap();
    chunk[start + 2..start + 5].to_string()
}

#[test]
fn ut_dom_080_ansi_background_codes_for_each_reason() {
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let l = lines(
        fen,
        &view(Some("e2e4"), false),
        BoardStyle::Ansi,
        Orientation::White,
    );
    // rank 1 is line 8: a1 dark (green 42), h1 light (white 47); rank 4 is line 5
    assert_eq!(bg(&l[8], 0), "42m");
    assert_eq!(bg(&l[8], 7), "47m");
    // the two last-move squares are yellow, whether the move started or ended there
    assert_eq!(bg(&l[5], 4), "43m"); // e4 (to)
    assert_eq!(bg(&l[7], 4), "43m"); // e2 (from)
    // rewound: blue on the same squares
    let rw = lines(
        fen,
        &view(Some("e2e4"), true),
        BoardStyle::Ansi,
        Orientation::White,
    );
    assert_eq!(bg(&rw[5], 4), "44m");
    assert_eq!(bg(&rw[7], 4), "44m");
    // check beats the others: red
    let chk = lines(
        "4k3/8/8/8/8/8/4r3/R3K3 w - - 0 1",
        &view(Some("e2e1"), true),
        BoardStyle::Ansi,
        Orientation::White,
    );
    assert_eq!(bg(&chk[8], 4), "41m");
    assert_eq!(bg(&chk[8], 0), "42m", "a1 is a plain dark square");
}

#[test]
fn ut_dom_080_only_the_king_of_the_side_to_move_is_marked_in_check() {
    // A white rook stands on a1, before the king on e1 in square order; only e1 is marked.
    let g = grid(
        "4k3/8/8/8/8/8/4r3/R3K3 w - - 0 1",
        &PLAIN,
        BoardStyle::Plain,
        Orientation::White,
    );
    let marked: Vec<(usize, usize)> = (0..8)
        .flat_map(|r| (0..8).map(move |c| (r, c)))
        .filter(|(r, c)| g.rows[*r][*c].highlight.check)
        .collect();
    assert_eq!(marked, vec![(7, 4)]);
    // not in check: nothing marked
    let g = grid(START, &PLAIN, BoardStyle::Plain, Orientation::White);
    assert!(g.rows.iter().flatten().all(|c| !c.highlight.check));
}
