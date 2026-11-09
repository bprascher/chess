//! UT-DOM-073 to 076, 107 (DD-DOM-016): `VariationTree`. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{MAX_NODES, NodeId, TreeError, VariationTree};
use chess_rules::{Position, parse_line};
use common::mvu;

fn tree() -> VariationTree {
    VariationTree::new(Position::standard(), 0)
}

#[test]
fn ut_dom_073_ana_2_add_navigate_branch_delete_and_reuse() {
    let mut t = tree();
    let root = t.root();
    let e4 = t.play(root, mvu("e2e4")).unwrap();
    let e5 = t.play(e4, mvu("e7e5")).unwrap();
    let nf3 = t.play(e5, mvu("g1f3")).unwrap();
    // navigate back and forward
    assert_eq!(t.parent(nf3).unwrap(), Some(e5));
    assert_eq!(t.parent(root).unwrap(), None);
    assert_eq!(t.first_child(root).unwrap(), Some(e4));
    assert_eq!(t.first_child(nf3).unwrap(), None);
    assert_eq!(t.children(e4).unwrap(), &[e5]);
    assert_eq!(
        t.line_to(nf3).unwrap(),
        vec![mvu("e2e4"), mvu("e7e5"), mvu("g1f3")]
    );
    assert_eq!(t.line_to(root).unwrap(), vec![]);
    assert_eq!(t.ply(nf3).unwrap(), 3);
    assert!(!t.is_empty());
    assert_eq!(
        (root.index(), e4.index(), e5.index(), nf3.index()),
        (0, 1, 2, 3)
    );
    // branch again at e4
    let c5 = t.play(e4, mvu("c7c5")).unwrap();
    assert_eq!(
        t.children(e4).unwrap(),
        &[e5, c5],
        "creation order, first child is main"
    );
    assert_eq!(t.first_child(e4).unwrap(), Some(e5));
    // the same move twice reuses the node
    assert_eq!(t.play(e4, mvu("e7e5")).unwrap(), e5);
    assert_eq!(t.play(root, mvu("e2e4")).unwrap(), e4);
    assert_eq!(t.children(e4).unwrap().len(), 2);
    // the position of a node
    assert_eq!(
        t.position(e4).unwrap().to_fen().as_str(),
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
    );
    // delete a subtree: ids of other nodes stay valid, deleted ids are NotFound
    t.delete(e5).unwrap();
    assert_eq!(t.children(e4).unwrap(), &[c5]);
    assert_eq!(t.parent(c5).unwrap(), Some(e4));
    assert!(matches!(t.position(e5), Err(TreeError::NotFound)));
    assert_eq!(t.line_to(nf3), Err(TreeError::NotFound));
    assert_eq!(t.play(nf3, mvu("b8c6")), Err(TreeError::NotFound));
    assert_eq!(t.delete(nf3), Err(TreeError::NotFound));
    // ids are never reused
    let again = t.play(e4, mvu("e7e5")).unwrap();
    assert_ne!(again, e5);
    assert_ne!(again, nf3);
    // an id that was never issued
    assert_eq!(t.parent(NodeId::from_index(9999)), Err(TreeError::NotFound));
}

#[test]
fn ut_dom_073_the_root_cannot_be_deleted() {
    let mut t = tree();
    assert_eq!(t.delete(t.root()), Err(TreeError::DeleteRoot));
}

#[test]
fn ut_dom_074_illegal_what_if_move_is_rejected_and_nothing_changes() {
    let mut t = tree();
    let root = t.root();
    assert!(matches!(
        t.play(root, mvu("e2e5")),
        Err(TreeError::IllegalMove(_))
    ));
    assert_eq!(t.len(), 1);
    assert!(t.children(root).unwrap().is_empty());
    // value semantics: the position the tree was built from is untouched
    let start = Position::standard();
    let mut t = VariationTree::new(start.clone(), 0);
    t.play(t.root(), mvu("e2e4")).unwrap();
    assert_eq!(start.to_fen(), Position::standard().to_fen());
    assert_eq!(t.position(t.root()).unwrap().to_fen(), start.to_fen());
}

#[test]
fn ut_dom_075_pgn_round_trip_with_variations() {
    let mut t = tree();
    let root = t.root();
    let e4 = t.play(root, mvu("e2e4")).unwrap();
    let e5 = t.play(e4, mvu("e7e5")).unwrap();
    let nf3 = t.play(e5, mvu("g1f3")).unwrap();
    t.play(nf3, mvu("b8c6")).unwrap();
    let c5 = t.play(e4, mvu("c7c5")).unwrap();
    t.play(c5, mvu("g1f3")).unwrap();
    t.play(root, mvu("d2d4")).unwrap();
    let text = t.to_pgn_text().unwrap();
    assert_eq!(text, "1. e4 (1. d4) 1... e5 (1... c5 2. Nf3) 2. Nf3 Nc6");
    // it is valid line text for chess-rules and the nodes agree with the tree
    assert_eq!(
        parse_line(&Position::standard(), &text).unwrap(),
        t.to_pgn_nodes()
    );
    let back = VariationTree::from_pgn_text(Position::standard(), 0, &text).unwrap();
    assert_eq!(back.to_pgn_text().unwrap(), text);
    assert_eq!(back.to_pgn_nodes(), t.to_pgn_nodes());
    // shape: same number of live nodes and the same lines
    assert_eq!(back.len(), t.len());
    // a main line only
    let mut line = tree();
    let n = line.play(line.root(), mvu("e2e4")).unwrap();
    line.play(n, mvu("e7e5")).unwrap();
    assert_eq!(line.to_pgn_text().unwrap(), "1. e4 e5");
    assert_eq!(tree().to_pgn_text().unwrap(), "");
    assert!(tree().to_pgn_nodes().is_empty());
    // nested variation
    let nested = "1. e4 e5 (1... c5 2. Nf3 (2. Nc3) 2... d6) 2. Nf3";
    let t2 = VariationTree::from_pgn_text(Position::standard(), 0, nested).unwrap();
    assert_eq!(t2.to_pgn_text().unwrap(), nested);
    // bad text and an illegal line are errors, not panics
    assert_eq!(
        VariationTree::from_pgn_text(Position::standard(), 0, "1. e4 e4").err(),
        Some(TreeError::BadText)
    );
    assert_eq!(
        VariationTree::from_pgn_text(Position::standard(), 0, "(((").err(),
        Some(TreeError::BadText)
    );
}

#[test]
fn ut_dom_075_a_tree_rooted_mid_game_numbers_from_its_root_position() {
    let after_e4 = Position::standard().play(mvu("e2e4")).unwrap();
    let mut t = VariationTree::new(after_e4.clone(), 1);
    let n = t.play(t.root(), mvu("e7e5")).unwrap();
    assert_eq!(t.ply(n).unwrap(), 2);
    assert_eq!(t.to_pgn_text().unwrap(), "1... e5");
    let back = VariationTree::from_pgn_text(after_e4, 1, "1... e5").unwrap();
    assert_eq!(back.len(), t.len());
}

#[test]
fn ut_dom_076_max_nodes_returns_full() {
    // shuffle knights to make a long legal line, then branch until the arena is full
    let mut t = tree();
    let mut at = t.root();
    let cycle = ["g1f3", "g8f6", "f3g1", "f6g8"];
    let mut count = 1; // the root
    'outer: loop {
        for u in cycle {
            if count == MAX_NODES {
                break 'outer;
            }
            at = t.play(at, mvu(u)).unwrap();
            count += 1;
        }
    }
    assert_eq!(t.len(), MAX_NODES);
    let next = t.position(at).unwrap().legal_moves()[0];
    assert_eq!(t.play(at, next), Err(TreeError::Full));
    // a move that already exists is not a new node, so it still succeeds when full
    let root = t.root();
    assert!(t.play(root, mvu("g1f3")).is_ok());
    // deleting frees room (live nodes are counted), ids are still not reused
    let first = t.first_child(root).unwrap().unwrap();
    t.delete(first).unwrap();
    assert!(t.len() < MAX_NODES);
    assert!(t.play(root, mvu("e2e4")).is_ok());
}

#[test]
fn ut_dom_107_cr_9_4_saved_line_refuses_a_branch_from_the_final_position() {
    let mut t = VariationTree::new(Position::standard().play(mvu("e2e4")).unwrap(), 1);
    let a = t.play(t.root(), mvu("c7c5")).unwrap();
    let b = t.play(a, mvu("g1f3")).unwrap();
    // the game has 3 plies: the branch starts at ply 1, before the last ply
    let s = t.saved_line(b, 3).unwrap();
    assert_eq!(s.anchor_ply, 1);
    assert_eq!(s.text, "1... c5 2. Nf3");
    // the game has exactly 1 ply: the branch starts from the final position, so it is refused
    assert_eq!(t.saved_line(b, 1), Err(TreeError::AtFinalPosition));
    // and beyond the end
    assert_eq!(t.saved_line(b, 0), Err(TreeError::AtFinalPosition));
    // the boundary: a branch rooted at ply 2 of a 3-ply game is allowed
    let mid = Position::standard()
        .play(mvu("e2e4"))
        .unwrap()
        .play(mvu("e7e5"))
        .unwrap();
    let mut u = VariationTree::new(mid, 2);
    let n = u.play(u.root(), mvu("g1f3")).unwrap();
    assert_eq!(u.saved_line(n, 3).unwrap().anchor_ply, 2);
    assert_eq!(u.saved_line(n, 2), Err(TreeError::AtFinalPosition));
    // an empty line (the root) has nothing to save
    assert_eq!(t.saved_line(t.root(), 3), Err(TreeError::EmptyLine));
    // a deleted node
    t.delete(a).unwrap();
    assert_eq!(t.saved_line(b, 3), Err(TreeError::NotFound));
}

#[test]
fn ut_dom_100_tree_errors_render_plain_messages() {
    for e in [
        TreeError::Full,
        TreeError::NotFound,
        TreeError::DeleteRoot,
        TreeError::AtFinalPosition,
        TreeError::EmptyLine,
        TreeError::BadText,
    ] {
        let m = e.to_string();
        assert!(
            !m.is_empty() && !m.contains('/') && !m.contains("e2"),
            "{m}"
        );
    }
}

/// A chain of `levels` nested variations: at each node the first child is a leaf (the main line
/// of that node) and the second child continues the chain, so each level is a variation one
/// level deeper than the one before. Built only through `play`.
fn nested_tree(levels: usize) -> VariationTree {
    let mut t = tree();
    let mut at = t.root();
    for _ in 0..levels {
        let moves = t.position(at).unwrap().legal_moves();
        let leaf = *moves.first().unwrap();
        let cont = *moves.get(1).unwrap();
        t.play(at, leaf).unwrap();
        at = t.play(at, cont).unwrap();
    }
    t
}

#[test]
fn df_03_deep_tree_text_is_limited_by_the_rules_writer_depth() {
    // GATE-103 rework DF-03: the writer in chess-rules refuses variations nested deeper than
    // DEFAULT_MAX_VARIATION_DEPTH; `to_pgn_text` reports that as `BadText` (documented on the
    // method). Cause pinned: the boundary is that constant, not MAX_NODES.
    let limit = chess_rules::DEFAULT_MAX_VARIATION_DEPTH;
    let at_limit = nested_tree(limit);
    let text = at_limit.to_pgn_text().unwrap();
    // it round-trips: the reader's default depth is the same constant
    let back = VariationTree::from_pgn_text(Position::standard(), 0, &text).unwrap();
    assert_eq!(back.len(), at_limit.len());
    let over = nested_tree(limit + 1);
    assert!(over.len() < MAX_NODES, "inside the node limit");
    assert_eq!(over.to_pgn_text(), Err(TreeError::BadText));
    // the tree itself and its single lines are not limited by that depth
    let deep = nested_tree(limit * 4);
    assert!(deep.len() < MAX_NODES);
    assert_eq!(deep.to_pgn_nodes().len(), 1);
}
