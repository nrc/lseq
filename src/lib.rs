#![feature(const_vec_new)]

use bit_vec::BitVec;
use rand::{thread_rng, Rng};
use serde_derive::{Serialize, Deserialize};

const INITIAL_WIDTH: u64 = 32;
// FIXME currently cannot be customised.
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

    pub fn id(&mut self, lower_bound: &Id, upper_bound: &Id) -> Id {
        assert!(lower_bound.depth() > 0 && upper_bound.depth() > 0);
        assert!(lower_bound < upper_bound);

        let mut depth = 0;
        loop {
            if lower_bound.index[depth] < upper_bound.index[depth] {
                // lower_bound and upper_bound are diverging.
                return self.id_neither_depth(depth + 1, lower_bound, upper_bound);
            }
            assert!(lower_bound.index[depth] == upper_bound.index[depth]);

            depth += 1;
            // Up to depth, lower_bound == upper_bound

            if depth == lower_bound.depth() {
                if depth == upper_bound.depth() {
                    return self.id_eq_depth(depth, lower_bound, upper_bound);
                } else {
                    return self.id_lower_depth(depth, lower_bound, upper_bound);
                }
            }
            // lower_bound.depth() > depth && lower_bound < upper_bound
            // therefore upper_bound.depth() > depth
            assert!(upper_bound.depth() > depth);

            // Both bounds have more levels; continue.
        }
    }

    // Invariant for all these functions: either a bound has > depth levels or
    // lower.index[depth - 1] < upper.index[depth - 1]
    // and both bounds have at least depth levels
    fn id_eq_depth(&mut self, depth: usize, lower_bound: &Id, upper_bound: &Id) -> Id {
        let level_lower_bound = lower_bound.index[depth - 1];
        let level_upper_bound = upper_bound.index[depth - 1];

        assert!(level_upper_bound != level_lower_bound);
        if level_upper_bound - level_lower_bound == 1 {
            // No room between lower_bound and upper_bound
            // go up a level on lower_bound either existing or adding a level
            // if we ever add a level 0, we must immediately go up a level, i.e., 0 is never a leaf
            if lower_bound.depth() == depth {
                let width = self.width_at(depth);
                let new_index = self.pick_index(depth, 0, width);
                if new_index == 0 {
                    // TODO need to add another level
                }
                return self.with_new_index(lower_bound, new_index);
            } else {
                let rhs = self.replace_index(lower_bound, self.width_at(depth));
                // FIXME could start the loop at depth = rhs.index.len() - 1
                return self.id(lower_bound, &rhs);
            }
        }

        // Invariant: there is room to add an id between lower_bound and upper_bound

        let new_index = self.pick_index(depth, level_lower_bound, level_upper_bound);
        // TODO only works if lower_bound.depth() == depth
        self.replace_index(lower_bound, new_index)
    }

    // If upper_bound.index[depth + 1] > 0 then pick an id between 0 and upper_bound
    // else go up a level on the right (which must exist)
    fn id_lower_depth(&mut self, depth: usize, lower_bound: &Id, upper_bound: &Id) -> Id {
        assert!(lower_bound.depth() == depth && upper_bound.depth() > depth);
        let mut lhs = lower_bound.clone();
        lhs.index.push(0);
        // FIXME A little bit inefficient since we'll rewalk depth levels.
        self.id(&lhs, upper_bound)
    }

    fn id_neither_depth(&mut self, depth: usize, lower_bound: &Id, upper_bound: &Id) -> Id {
        self.id_eq_depth(depth, lower_bound, upper_bound)
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

    fn pick_index(&mut self, depth: usize, lower_bound: u64, upper_bound: u64) -> u64 {
        assert!(lower_bound + 1 < upper_bound, "{} < {}", lower_bound + 1, upper_bound);
        if self.level_direction(depth) {
            Self::pick_index_upper_boundary(lower_bound, upper_bound)
        } else {
            Self::pick_index_lower_boundary(lower_bound, upper_bound)
        }
    }

    fn pick_index_lower_boundary(lower_bound: u64, upper_bound: u64) -> u64 {
        let mut boundary = lower_bound + DEFAULT_BOUNDARY;
        if boundary > upper_bound {
            boundary = upper_bound;
        }
        random_range(lower_bound, boundary)
    }

    fn pick_index_upper_boundary(lower_bound: u64, upper_bound: u64) -> u64 {
        let mut boundary = upper_bound.saturating_sub(DEFAULT_BOUNDARY + 1);
        if boundary < lower_bound {
            boundary = lower_bound;
        }
        random_range(boundary, upper_bound)
    }

    fn width_at(&self, depth: usize) -> u64 {
        self.initial_width * 2_u64.pow(depth as u32)
    }

    fn with_new_index(&self, id: &Id, new_index: u64) -> Id {
        let mut new_id = id.clone();
        new_id.node = self.id;
        new_id.index.push(new_index);
        new_id
    }

    fn replace_index(&self, id: &Id, new_index: u64) -> Id {
        let mut new_id = id.clone();
        new_id.node = self.id;
        new_id.index[id.index.len() - 1] = new_index;
        new_id
    }

    pub fn begin(&self) -> Id {
        Id {
            index: vec![],
            node: self.id,
        }
    }

    // FIXME wouldn't need an end if l bound and r bound could be equal
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
    fn test_width_at() {
        let mut node = Node::new(NodeId::new(0));
        node.initial_width = 5;
        assert!(node.width_at(0) == 5);
        assert!(node.width_at(1) == 10);
        assert!(node.width_at(2) == 20);
        assert!(node.width_at(3) == 40);
        assert!(node.width_at(4) == 80);
    }

    #[test]
    fn test_with_new_index() {
        let node = Node::new(NodeId::new(0));

        let id = Id { index: vec![], node: NodeId::new(42) };
        let new_id = node.with_new_index(&id, 6);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.index.len() == 1);
        assert!(new_id.index[0] == 6);

        let id = Id { index: vec![4], node: NodeId::new(0) };
        let new_id = node.with_new_index(&id, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.index.len() == 2);
        assert!(new_id.index[0] == 4);
        assert!(new_id.index[1] == 0);
    }

    #[test]
    fn test_replace_index() {
        let node = Node::new(NodeId::new(0));

        let id = Id { index: vec![4], node: NodeId::new(0) };
        let new_id = node.replace_index(&id, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.index.len() == 1);
        assert!(new_id.index[0] == 0);
    }

    #[test]
    fn test_pick_index() {
        let mut node = Node::new(NodeId::new(0));
        for i in 0..20 {
            for j in (i+2)..20 {
                for depth in 0..10 {
                    for _ in 0..1000 {
                        let new_index = node.pick_index(depth, i, j);
                        assert!(new_index > i && new_index < j);
                    }
                }
            }
        }
    }

    #[test]
    fn basic_unique() {
        let mut node = Node::new(NodeId::new(0));

        // let a = node.id(&node.begin(), &node.end());
        // let b = node.id(&a, &node.end());

        // assert!(a != b);
    }
}
