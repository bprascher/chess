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
        TreeError::TooDeep,
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

/// Plays one more level of `nested_tree`: the leaf (first child) then the continuation (second
/// child). Returns the result of the continuation.
fn one_more_level(t: &mut VariationTree, at: NodeId) -> Result<NodeId, TreeError> {
    let moves = t.position(at).unwrap().legal_moves();
    t.play(at, *moves.first().unwrap()).unwrap();
    t.play(at, *moves.get(1).unwrap())
}

fn deepest(t: &VariationTree) -> NodeId {
    let mut at = t.root();
    loop {
        let kids = t.children(at).unwrap();
        match kids.last() {
            Some(&k) => at = k,
            None => return at,
        }
    }
}

#[test]
fn df_03_d7_variation_depth_64_is_accepted_and_65_is_refused_with_too_deep() {
    // Desk review D7 (approved 2026-11-09): `play` refuses a variation that would nest deeper
    // than chess_rules::DEFAULT_MAX_VARIATION_DEPTH, so every tree that can exist can be exported.
    let limit = chess_rules::DEFAULT_MAX_VARIATION_DEPTH;
    assert_eq!(limit, 64, "the SDD names 64");
    let mut t = nested_tree(limit);
    let at = deepest(&t);
    let before = t.len();
    assert_eq!(one_more_level(&mut t, at), Err(TreeError::TooDeep));
    // the leaf (first child, not a variation) was added, the refused move was not
    assert_eq!(t.len(), before + 1, "a refused play changes nothing");
    let again = t.position(at).unwrap().legal_moves();
    assert_eq!(
        t.play(at, *again.get(1).unwrap()),
        Err(TreeError::TooDeep),
        "still refused"
    );
}

#[test]
fn df_03_d7_every_tree_built_at_the_maximum_exports_and_round_trips() {
    let limit = chess_rules::DEFAULT_MAX_VARIATION_DEPTH;
    let mut t = nested_tree(limit);
    let at = deepest(&t);
    // the main continuation of the deepest node is not a deeper variation
    let moves = t.position(at).unwrap().legal_moves();
    let cont = t.play(at, *moves.first().unwrap()).unwrap();
    let moves2 = t.position(cont).unwrap().legal_moves();
    t.play(cont, *moves2.first().unwrap()).unwrap();
    // ...and so is a refused sibling: the tree stays exportable
    assert_eq!(t.play(at, *moves.get(1).unwrap()), Err(TreeError::TooDeep));
    let text = t.to_pgn_text().expect("exports");
    let back = VariationTree::from_pgn_text(Position::standard(), 0, &text).unwrap();
    assert_eq!(back.len(), t.len());
    assert_eq!(back.to_pgn_text().unwrap(), text, "round trip");
    // nested_tree(limit * 4) can no longer be built through `play`
    let mut deep = nested_tree(limit);
    let mut at = deepest(&deep);
    for _ in 0..3 * limit {
        match one_more_level(&mut deep, at) {
            Ok(next) => at = next,
            Err(e) => {
                assert_eq!(e, TreeError::TooDeep);
                break;
            }
        }
    }
    assert!(deep.to_pgn_text().is_ok());
}

#[test]
fn df_03_d7_deleting_the_first_child_makes_room_again() {
    // Depth is the number of non-first edges on the path. Deleting a first child promotes the
    // next sibling to the main line, one level shallower.
    let limit = chess_rules::DEFAULT_MAX_VARIATION_DEPTH;
    let mut t = nested_tree(limit - 1);
    let at = deepest(&t);
    let leaf = *t
        .children(t.parent(at).unwrap().unwrap())
        .unwrap()
        .first()
        .unwrap();
    // at depth limit-1; one more level is fine (depth 64) ...
    let top = one_more_level(&mut t, at).unwrap();
    // ... and one more is refused
    assert_eq!(one_more_level(&mut t, top), Err(TreeError::TooDeep));
    // delete the first (leaf) child of the parent of `at`: `at` becomes the main line there
    let parent = t.parent(at).unwrap().unwrap();
    t.delete(leaf).unwrap();
    assert_eq!(t.first_child(parent).unwrap(), Some(at));
    // `top`'s line is now one level shallower: another level fits
    assert!(one_more_level(&mut t, top).is_ok());
    assert!(t.to_pgn_text().is_ok());
}

#[test]
fn df_03_d7_reading_text_nested_deeper_than_the_limit_is_still_bad_text() {
    // The reader and the tree share the limit: 64 reads, 65 does not.
    let limit = chess_rules::DEFAULT_MAX_VARIATION_DEPTH;
    let text = nested_tree(limit).to_pgn_text().unwrap();
    assert!(VariationTree::from_pgn_text(Position::standard(), 0, &text).is_ok());
    let over = format!("1. e4 {}1. d4 )", "(".repeat(limit + 1));
    assert_eq!(
        VariationTree::from_pgn_text(Position::standard(), 0, &over).err(),
        Some(TreeError::BadText)
    );
}
