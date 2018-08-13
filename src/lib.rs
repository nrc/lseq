#![feature(const_vec_new)]

use bit_vec::BitVec;
use rand::{thread_rng, Rng};
use serde_derive::{Serialize, Deserialize};

const INITIAL_WIDTH: u64 = 32;
// TODO currently cannot be customised.
const DEFAULT_BOUNDARY: u64 = 10;

/// Generates unique ids. There should be one `Node` per replicated instance.
pub struct Node {
    id: NodeId,
    // True = upper, false = lower
    directions: BitVec,
    initial_width: u64,
}

impl Node {
    pub fn new(id: NodeId) -> Node {
        Node {
            id,
            directions: BitVec::new(),
            initial_width: INITIAL_WIDTH,
        }
    }


    fn level_direction(&mut self, level: usize) -> bool {
        if level < self.directions.len() {
            self.directions[level]
        } else if level == self.directions.len() {
            let result = random_bool();
            self.directions.push(result);
            result
        } else {
            panic!("Skipped a level");
        }
    }

    pub fn begin(&self) -> Id {
        Id {
            index: vec![],
            node: self.id,
        }
    }

    // TODO wouldn't need an end if l bound and r bound could be equal
    pub fn end(&self) -> Id {
        Id {
            index: vec![(self.initial_width - 1) as u64],
            node: self.id,
        }
    }
}

/// Identifies a `Node`. Supplied by the client and must be globally unique.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub struct NodeId(u32);

impl NodeId {
    pub fn new(i: u32) -> NodeId {
        NodeId(i)
    }
}

/// An LSeq Id, created by a `Node`.
// FIXME could optimise Eq/ParialEq by comparing pointer value of index
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Id {
    // Ordering of fields is for derive(Ord).

    // Index into the tree of identifiers. The nth entry in the `Vec` specifies
    // a node in the nth level of the tree.
    index: Vec<u64>,
    // The `Node` which created this id.
    node: NodeId,
}

impl Id {
    fn depth(&self) -> usize {
        self.index.len()
    }
}

fn random_bool() -> bool {
    let mut rng = thread_rng();
    rng.gen()
}

// Exclusive above and below.
fn random_range(l: u64, u: u64) -> u64 {
    assert!(l + 1 < u, "{} < {}", l + 1, u);
    let mut rng = thread_rng();
    rng.gen_range(l + 1, u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_range() {
        for i in 0..100 {
            let r = random_range(i, i * 2 + 2);
            assert!(r > i && r < i * 2 + 2);
        }
    }

    #[test]
    fn test_id_props() {
        let a = Id { index: vec![], node: NodeId(0) };
        let b = Id { index: vec![], node: NodeId(2) };
        let c = Id { index: vec![5, 32, 100, 2], node: NodeId(2) };
        let d = Id { index: vec![5, 32, 100, 2], node: NodeId(2) };
        let e = Id { index: vec![5, 32, 100, 2], node: NodeId(3) };
        let f = Id { index: vec![5, 32, 100], node: NodeId(2) };
        let g = Id { index: vec![4, 40], node: NodeId(0) };

        // Equality, inequality
        assert!(a == a);
        assert!(g == g);
        assert!(c == d);
        assert!(a != b);
        assert!(c != f);
        assert!(d != e);

        // Ordering
        assert!(a < b);
        assert!(a < c);
        assert!(d < e);
        assert!(g < f);
        assert!(b < f);
        assert!(c < e);
        assert!(f < c);
    }

    #[test]
    fn test_level_direction() {
        let mut node = Node::new(NodeId::new(0));
        let a = node.level_direction(0);
        let b = node.level_direction(1);
        let c = node.level_direction(2);
        let d = node.level_direction(3);

        let a_ = node.level_direction(0);
        let b_ = node.level_direction(1);
        let c_ = node.level_direction(2);
        let d_ = node.level_direction(3);

        assert!(a == a_);
        assert!(b == b_);
        assert!(c == c_);
        assert!(d == d_);

        // Test that eventually we'll get both values.
        let mut f = false;
        let mut t = false;
        let mut i = 0;
        loop {
            let a = node.level_direction(i);
            if a {
                t = true;
            } else {
                f = true;
            }

            if t && f {
                return;
            }

            i += 1;
        }
    }

    #[test]
    #[should_panic]
    fn test_bad_level_direction_1() {
        let mut node = Node::new(NodeId::new(0));
        node.level_direction(1);
    }

    #[test]
    #[should_panic]
    fn test_bad_level_direction_2() {
        let mut node = Node::new(NodeId::new(0));
        node.level_direction(0);
        node.level_direction(1);
        node.level_direction(6);
    }

    #[test]
    fn basic_unique() {
        let mut node = Node::new(NodeId::new(0));

        // let a = node.id(&node.begin(), &node.end());
        // let b = node.id(&a, &node.end());

        // assert!(a != b);
    }
}
