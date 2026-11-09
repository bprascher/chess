//! DD-DOM-016: `VariationTree`, an arena of positions for what-if play and saved variations
//! (SDD 3.11.10). `NodeId` is an index and is never reused: a deleted node stays as a
//! tombstone, so ids held by a UI stay valid. The consumer is the PGN mapping (XC-24): the
//! tree is written and read through `chess-rules` line text.

use chess_rules::{Move, PgnNode, Position, format_line, parse_line};

use crate::error::TreeError;

/// The most live nodes a tree may hold (`ESTIMATE`, SDD 3.11.10).
pub const MAX_NODES: usize = 4096;

/// A node of a `VariationTree`. Never reused after `delete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    /// The id for an arena index (ids that were never issued answer `NotFound`).
    pub fn from_index(index: u32) -> NodeId {
        NodeId(index)
    }

    pub fn index(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
struct Node {
    /// The move that led here; `None` for the root.
    mv: Option<Move>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    position: Position,
    alive: bool,
}

/// One saved variation: the ply of the position it branches from and its line text (RULE-6).
/// The store adds `created_at`; no Player is recorded (Q-SE-9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedLine {
    pub anchor_ply: u32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct VariationTree {
    root_ply: u32,
    nodes: Vec<Node>,
    live: usize,
}

impl VariationTree {
    /// A tree whose root is `root`, the position after `root_ply` plies of the game.
    pub fn new(root: Position, root_ply: u32) -> VariationTree {
        VariationTree {
            root_ply,
            nodes: vec![Node {
                mv: None,
                parent: None,
                children: Vec::new(),
                position: root,
                alive: true,
            }],
            live: 1,
        }
    }

    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Live nodes, the root included.
    pub fn len(&self) -> usize {
        self.live
    }

    /// Never true: the root cannot be deleted.
    pub fn is_empty(&self) -> bool {
        false
    }

    fn node(&self, id: NodeId) -> Result<&Node, TreeError> {
        let index = usize::try_from(id.0).map_err(|_| TreeError::NotFound)?;
        self.nodes
            .get(index)
            .filter(|n| n.alive)
            .ok_or(TreeError::NotFound)
    }

    pub fn parent(&self, node: NodeId) -> Result<Option<NodeId>, TreeError> {
        Ok(self.node(node)?.parent)
    }

    /// The main continuation: the first move played from `node`.
    pub fn first_child(&self, node: NodeId) -> Result<Option<NodeId>, TreeError> {
        Ok(self.node(node)?.children.first().copied())
    }

    /// In creation order.
    pub fn children(&self, node: NodeId) -> Result<&[NodeId], TreeError> {
        Ok(&self.node(node)?.children)
    }

    pub fn position(&self, node: NodeId) -> Result<&Position, TreeError> {
        Ok(&self.node(node)?.position)
    }

    /// The ply of the position at `node`: the root's ply plus the depth.
    pub fn ply(&self, node: NodeId) -> Result<u32, TreeError> {
        let depth = u32::try_from(self.line_to(node)?.len()).unwrap_or(u32::MAX);
        Ok(self.root_ply.saturating_add(depth))
    }

    /// The moves from the root to `node`.
    pub fn line_to(&self, node: NodeId) -> Result<Vec<Move>, TreeError> {
        let mut moves = Vec::new();
        let mut at = self.node(node)?;
        while let Some(mv) = at.mv {
            moves.push(mv);
            at = match at.parent {
                Some(p) => self.node(p)?,
                None => break,
            };
        }
        moves.reverse();
        Ok(moves)
    }

    /// Plays `mv` at `at`. The same move twice returns the existing child. An illegal move is an
    /// error and changes nothing.
    pub fn play(&mut self, at: NodeId, mv: Move) -> Result<NodeId, TreeError> {
        let here = self.node(at)?;
        for &child in &here.children {
            if self.node(child)?.mv == Some(mv) {
                return Ok(child);
            }
        }
        let position = here.position.play(mv)?;
        if self.live >= MAX_NODES {
            return Err(TreeError::Full);
        }
        let id = NodeId(u32::try_from(self.nodes.len()).map_err(|_| TreeError::Full)?);
        self.nodes.push(Node {
            mv: Some(mv),
            parent: Some(at),
            children: Vec::new(),
            position,
            alive: true,
        });
        let index = usize::try_from(at.0).map_err(|_| TreeError::NotFound)?;
        if let Some(parent) = self.nodes.get_mut(index) {
            parent.children.push(id);
        }
        self.live += 1;
        Ok(id)
    }

    /// Removes `node` and its subtree. The root cannot be deleted.
    pub fn delete(&mut self, node: NodeId) -> Result<(), TreeError> {
        let parent = self.node(node)?.parent.ok_or(TreeError::DeleteRoot)?;
        let mut stack = vec![node];
        while let Some(id) = stack.pop() {
            let index = usize::try_from(id.0).map_err(|_| TreeError::NotFound)?;
            if let Some(n) = self.nodes.get_mut(index) {
                if n.alive {
                    n.alive = false;
                    self.live -= 1;
                }
                stack.append(&mut n.children);
            }
        }
        let index = usize::try_from(parent.0).map_err(|_| TreeError::NotFound)?;
        if let Some(p) = self.nodes.get_mut(index) {
            p.children.retain(|c| *c != node);
        }
        Ok(())
    }

    /// The tree as `PgnNode`s: the first child at each position is the main line, the other
    /// children are variations attached to the first child's node. Nesting depth is bounded by
    /// `MAX_NODES`.
    pub fn to_pgn_nodes(&self) -> Vec<PgnNode> {
        self.line_nodes(self.root())
    }

    fn line_nodes(&self, from: NodeId) -> Vec<PgnNode> {
        let mut out = Vec::new();
        let mut at = from;
        while let Ok(node) = self.node(at) {
            let Some((&first, rest)) = node.children.split_first() else {
                break;
            };
            let Some(mv) = self.node(first).ok().and_then(|n| n.mv) else {
                break;
            };
            let mut pgn = PgnNode::new(mv);
            for &alt in rest {
                pgn.variations.push(self.branch_nodes(alt));
            }
            out.push(pgn);
            at = first;
        }
        out
    }

    /// A variation starting with the move that leads to `first`, then its main line.
    fn branch_nodes(&self, first: NodeId) -> Vec<PgnNode> {
        let Some(mv) = self.node(first).ok().and_then(|n| n.mv) else {
            return Vec::new();
        };
        let mut head = PgnNode::new(mv);
        let mut tail = self.line_nodes(first);
        let mut out = Vec::with_capacity(tail.len() + 1);
        // `head` has no variations of its own: the siblings were attached by the caller.
        head.variations.clear();
        out.push(head);
        out.append(&mut tail);
        out
    }

    /// The tree as line text from the root position (`chess-rules::format_line`). The nodes are
    /// legal by construction, so an illegal move cannot be the cause. The error arm is reached
    /// for a tree whose variations nest deeper than `chess_rules::DEFAULT_MAX_VARIATION_DEPTH`
    /// (64): the writer refuses it and this method reports it as `BadText`, well inside
    /// `MAX_NODES` (GATE-103 rework DF-03). `from_pgn_text` applies the same limit on reading.
    /// A distinct error for that case would change `TreeError`, and waits for an owner decision.
    pub fn to_pgn_text(&self) -> Result<String, TreeError> {
        let root = &self.node(self.root())?.position;
        format_line(root, &self.to_pgn_nodes()).map_err(|_| TreeError::BadText)
    }

    /// Reads line text (variations included) from `root`, the position after `root_ply` plies.
    /// Text with variations nested deeper than `chess_rules::DEFAULT_MAX_VARIATION_DEPTH` is
    /// `BadText`, like any other text the reader refuses.
    pub fn from_pgn_text(
        root: Position,
        root_ply: u32,
        text: &str,
    ) -> Result<VariationTree, TreeError> {
        let nodes = parse_line(&root, text).map_err(|_| TreeError::BadText)?;
        let mut tree = VariationTree::new(root, root_ply);
        let root_id = tree.root();
        tree.add_line(root_id, &nodes)?;
        Ok(tree)
    }

    fn add_line(&mut self, from: NodeId, nodes: &[PgnNode]) -> Result<(), TreeError> {
        let mut at = from;
        for node in nodes {
            let next = self.play(at, node.mv)?;
            for variation in &node.variations {
                self.add_line(at, variation)?;
            }
            at = next;
        }
        Ok(())
    }

    /// The line from the root to `node` as one saved variation (RULE-6). The branch is anchored
    /// at the root's ply. A branch whose root is the final position of the game, or beyond it,
    /// is refused: it is a variation of no move and PGN has no place for it (CR-9.4).
    pub fn saved_line(&self, node: NodeId, game_plies: u32) -> Result<SavedLine, TreeError> {
        let line = self.line_to(node)?;
        if self.root_ply >= game_plies {
            return Err(TreeError::AtFinalPosition);
        }
        if line.is_empty() {
            return Err(TreeError::EmptyLine);
        }
        let nodes: Vec<PgnNode> = line.into_iter().map(PgnNode::new).collect();
        let root = &self.node(self.root())?.position;
        let text = format_line(root, &nodes).map_err(|_| TreeError::BadText)?;
        Ok(SavedLine {
            anchor_ply: self.root_ply,
            text,
        })
    }
}
