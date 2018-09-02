#![feature(const_vec_new)]

use bit_vec::BitVec;
use rand::{thread_rng, Rng};
use serde_derive::{Serialize, Deserialize};

const INITIAL_WIDTH: u64 = 16;
// FIXME currently cannot be customised.
const DEFAULT_BOUNDARY: u64 = 10;

/// Generates unique ids. There should be one `Node` per replicated instance.
pub struct Node {
    id: NodeId,
    // True = upper, false = lower
    directions: BitVec,
    initial_width: u64,
}

// TODO IdSlice?

impl Node {
    pub fn new(id: NodeId) -> Node {
        let mut result = Node {
            id,
            directions: BitVec::new(),
            initial_width: INITIAL_WIDTH,
        };
        result.level_direction(0);
        result
    }

    pub fn new_id_with_bounds(&mut self, lower_bound: &Id, upper_bound: &Id) -> Id {
        assert!(lower_bound.depth() > 0);
        assert!(upper_bound.depth() > 0);
        assert!(lower_bound < upper_bound, "{:?} >= {:?}", lower_bound, upper_bound);

        // This loop walks up the bounds in tandem until one runs out of levels, or
        // the two are diverging.
        let mut level = 0;
        loop {
            assert!(lower_bound.depth() > level);
            assert!(upper_bound.depth() > level);
            if level == lower_bound.depth() - 1 || level == upper_bound.depth() - 1 {
                return self.new_id_at_level_bounded(level, lower_bound, upper_bound);
            }

            if lower_bound.indices[level] < upper_bound.indices[level] {
                // lower_bound and upper_bound are diverging.
                return self.new_id_at_level_bounded(level, lower_bound, upper_bound)
            }
            assert!(lower_bound.indices[level] == upper_bound.indices[level]);

            level += 1;
        }
    }

    fn new_id_at_level_bounded(&mut self, level: usize, lower_bound: &Id, upper_bound: &Id) -> Id {
        assert!(lower_bound.depth() > level && upper_bound.depth() > level);
        let level_lower_bound = lower_bound.indices[level];
        let level_upper_bound = upper_bound.indices[level];

        if level_lower_bound + 1 < level_upper_bound {
            // there is room to add an id between lower_bound and upper_bound
            let new_index = self.pick_index(level, level_lower_bound, level_upper_bound);
            return self.truncate_and_replace_index(lower_bound, level, new_index);
        }

        if level_lower_bound <= level_upper_bound {
            assert!(level_lower_bound + 1 >= level_upper_bound);
            if lower_bound.depth() > level + 1 || upper_bound.depth() == level + 1 {
                return self.new_id_at_level_bounded_below(level + 1, lower_bound);
            }
        }

        assert!((lower_bound.depth() == level + 1 || level_lower_bound < level_upper_bound) && upper_bound.depth() > level + 1);
        let lhs = self.append_index(lower_bound, 0);
        self.new_id_at_level_bounded(level + 1, &lhs, upper_bound)
    }

    fn new_id_at_level_bounded_below(&mut self, level: usize, lower_bound: &Id) -> Id {
        assert!(lower_bound.depth() >= level);
        let width = self.width_at(level);
        if lower_bound.depth() == level {
            let new_index = self.pick_index(level, 0, width);
            self.append_index(&lower_bound, new_index)
        } else {
            let rhs = self.truncate_and_replace_index(lower_bound, level, width - 1);
            self.new_id_at_level_bounded(level, lower_bound, &rhs)
        }
    }

    fn level_direction(&mut self, level: usize) -> bool {
        while level >= self.directions.len() {
            let result = random_bool();
            self.directions.push(result);
        }

        return self.directions[level];
    }

    fn pick_index(&mut self, level: usize, lower_bound: u64, upper_bound: u64) -> u64 {
        assert!(lower_bound + 1 < upper_bound, "{} < {}", lower_bound + 1, upper_bound);
        if self.level_direction(level) {
            let mut boundary = upper_bound.saturating_sub(DEFAULT_BOUNDARY + 1);
            if boundary < lower_bound {
                boundary = lower_bound;
            }
            random_range(boundary, upper_bound)
        } else {
            let mut boundary = lower_bound + DEFAULT_BOUNDARY;
            if boundary > upper_bound {
                boundary = upper_bound;
            }
            random_range(lower_bound, boundary)
        }
    }

    fn width_at(&self, level: usize) -> u64 {
        self.initial_width * 2_u64.pow(level as u32)
    }

    fn append_index(&self, id: &Id, new_index: u64) -> Id {
        // FIXME could be more efficient than clone here by making the new indices
        // have the capacity of id.indices.len() + 1.
        let mut new_id = id.clone();
        new_id.node = self.id;
        new_id.indices.push(new_index);
        new_id
    }

    fn truncate_and_replace_index(&self, id: &Id, level: usize, new_index: u64) -> Id {
        assert!(level < id.indices.len());
        let mut new_id = id.clone();
        new_id.node = self.id;
        new_id.indices[level] = new_index;
        new_id.indices.truncate(level + 1);
        new_id
    }

    pub fn begin(&self) -> Id {
        Id {
            indices: vec![0],
            node: self.id,
        }
    }

    // TODO wouldn't need an end if l bound and r bound could be equal
    // would consider bounds == to mean insert above
    pub fn end(&self) -> Id {
        Id {
            indices: vec![(self.initial_width - 1) as u64],
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
// FIXME could optimise Eq/ParialEq by comparing pointer value of indices
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Id {
    // Ordering of fields is for derive(Ord).

    // Indices into the tree of identifiers. The nth entry in the `Vec` specifies
    // a node in the nth level of the tree.
    indices: Vec<u64>,
    // The `Node` which created this id.
    node: NodeId,
}

impl Id {
    fn depth(&self) -> usize {
        self.indices.len()
    }
}

// FIXME add IdSlice to avoid passing around depth

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
    use std::collections::BTreeSet;

    #[test]
    fn test_random_range() {
        for i in 0..100 {
            let r = random_range(i, i * 2 + 2);
            assert!(r > i && r < i * 2 + 2);
        }
    }

    #[test]
    fn test_id_props() {
        let a = Id { indices: vec![], node: NodeId(0) };
        let b = Id { indices: vec![], node: NodeId(2) };
        let c = Id { indices: vec![5, 32, 100, 2], node: NodeId(2) };
        let d = Id { indices: vec![5, 32, 100, 2], node: NodeId(2) };
        let e = Id { indices: vec![5, 32, 100, 2], node: NodeId(3) };
        let f = Id { indices: vec![5, 32, 100], node: NodeId(2) };
        let g = Id { indices: vec![4, 40], node: NodeId(0) };

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
    fn test_append_index() {
        let node = Node::new(NodeId::new(0));

        let id = Id { indices: vec![], node: NodeId::new(42) };
        let new_id = node.append_index(&id, 6);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.indices.len() == 1);
        assert!(new_id.indices[0] == 6);

        let id = Id { indices: vec![4], node: NodeId::new(0) };
        let new_id = node.append_index(&id, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.indices.len() == 2);
        assert!(new_id.indices[0] == 4);
        assert!(new_id.indices[1] == 0);
    }

    #[test]
    fn test_truncate_and_replace_index() {
        let node = Node::new(NodeId::new(0));

        let id = Id { indices: vec![4], node: NodeId::new(0) };
        let new_id = node.truncate_and_replace_index(&id, 0, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.indices.len() == 1);
        assert!(new_id.indices[0] == 0);

        let id = Id { indices: vec![4, 5, 3, 2], node: NodeId::new(0) };
        let new_id = node.truncate_and_replace_index(&id, 1, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.indices.len() == 2);
        assert!(new_id.indices[0] == 4);
        assert!(new_id.indices[1] == 0);
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
    fn test_id_basic() {
        let mut node = Node::new(NodeId::new(0));

        for _ in 0..100 {
            let a = node.new_id_with_bounds(&node.begin(), &node.end());
            let b = node.new_id_with_bounds(&a, &node.end());

            assert!(a != node.begin());
            assert!(a != node.end());
            assert!(b != node.begin());
            assert!(b != node.end());

            assert!(node.begin() < a);
            assert!(a < b);
            assert!(b < node.end());
        }
    }

    #[test]
    fn test_id_random() {
        let mut rng = thread_rng();

        for _ in 0..100 {
            let mut node = Node::new(NodeId::new(0));
            let mut results = BTreeSet::new();
            results.insert(node.begin());
            results.insert(node.end());
            for _ in 0..200 {
                let mut index_0 = rng.gen_range(0, results.len());
                let mut index_1 = rng.gen_range(0, results.len());
                while index_0 == index_1 {
                    index_1 = rng.gen_range(0, results.len());
                }
                if index_0 > index_1 {
                    let temp = index_0;
                    index_0 = index_1;
                    index_1 = temp;
                }
                let id_0 = results.iter().nth(index_0).unwrap();
                let id_1 = results.iter().nth(index_1).unwrap();
                let new = node.new_id_with_bounds(id_0, id_1);
                assert!(&new != id_0);
                assert!(&new != id_1);
                assert!(&new > id_0);
                assert!(&new < id_1);
                results.insert(new);
            }
        }        
    }

    // TODO test that we're using the full widths available, and not more than that.
    // TODO test the edge cases which require loops.
    #[test]
    fn test_id_left() {
        for _ in 0..100 {
            let mut node = Node::new(NodeId::new(0));
            let first = node.begin();
            let mut prev = node.end();
            for _ in 0..200 {
                let new = node.new_id_with_bounds(&first, &prev);
                assert!(&new != &first);
                assert!(&new != &prev);
                assert!(&new > &first);
                assert!(&new < &prev);
                prev = new;
            }
        }
    }

    #[test]
    fn test_id_right() {
        for _ in 0..100 {
            let mut node = Node::new(NodeId::new(0));
            let last = node.end();
            let mut prev = node.begin();
            for _ in 0..200 {
                let new = node.new_id_with_bounds(&prev, &last);
                assert!(&new != &prev);
                assert!(&new != &last);
                assert!(&new > &prev);
                assert!(&new < &last);
                prev = new;
            }
        }
    }
}
