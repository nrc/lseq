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

// TODO refactoring
// better names - functions, index
// consistent on off by one on depth
// factor out some more functions
// IdSlice

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

    pub fn id(&mut self, lower_bound: &Id, upper_bound: &Id) -> Id {
        assert!(upper_bound.depth() > 0);
        assert!(lower_bound < upper_bound, "{:?} >= {:?}", lower_bound, upper_bound);

        let mut depth = 0;
        loop {
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

            if lower_bound.index[depth] < upper_bound.index[depth] {
                // lower_bound and upper_bound are diverging.
                return self.id_eq_depth(depth + 1, lower_bound, upper_bound)
            }
            assert!(lower_bound.index[depth] == upper_bound.index[depth]);

            depth += 1;
            // Up to depth, lower_bound == upper_bound

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
                assert!(new_index != 0);
                return self.append_index(&lower_bound, new_index);
            } else {
                assert!(lower_bound.depth() > depth);
                if level_lower_bound + 1 < level_upper_bound {
                    // There is room at depth for another index
                    let lhs = self.truncate_and_replace_index(lower_bound, depth, level_lower_bound + 1);
                    assert!(&lhs < upper_bound, "{:?} {:?} {}", lhs, upper_bound, depth);
                    return self.id(&lhs, upper_bound);
                } else {
                    // Iterate up levels until we find a level we can insert into.
                    let mut cur_depth = depth;
                    let mut width;
                    loop {
                        cur_depth += 1;
                        if lower_bound.depth() == cur_depth {
                            let width = self.width_at(cur_depth);
                            let new_index = self.pick_index(cur_depth, 0, width);
                            let new_id = self.append_index(&lower_bound, new_index);
                            assert!(new_index != 0);
                            return new_id;
                        }
                        width = self.width_at(cur_depth);
                        if lower_bound.index[cur_depth] < width - 1 {
                            break;
                        }
                    }

                    let rhs = self.truncate_and_replace_index(lower_bound, cur_depth, width - 1);
                    assert!(lower_bound < &rhs, "{:?} {:?} {}", lower_bound, rhs, cur_depth);
                    return self.id(lower_bound, &rhs);
                }
            }
        }

        // Invariant: there is room to add an id between lower_bound and upper_bound

        let new_index = self.pick_index(depth, level_lower_bound, level_upper_bound);
        self.truncate_and_replace_index(lower_bound, depth, new_index)
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

    fn append_index(&self, id: &Id, new_index: u64) -> Id {
        let mut new_id = id.clone();
        new_id.node = self.id;
        new_id.index.push(new_index);
        new_id
    }

    fn truncate_and_replace_index(&self, id: &Id, depth: usize, new_index: u64) -> Id {
        assert!(depth <= id.index.len());
        let mut new_id = id.clone();
        new_id.node = self.id;
        new_id.index[depth - 1] = new_index;
        new_id.index.truncate(depth);
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
        node.level_direction(2);
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
    fn test_append_index() {
        let node = Node::new(NodeId::new(0));

        let id = Id { index: vec![], node: NodeId::new(42) };
        let new_id = node.append_index(&id, 6);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.index.len() == 1);
        assert!(new_id.index[0] == 6);

        let id = Id { index: vec![4], node: NodeId::new(0) };
        let new_id = node.append_index(&id, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.index.len() == 2);
        assert!(new_id.index[0] == 4);
        assert!(new_id.index[1] == 0);
    }

    #[test]
    fn test_truncate_and_replace_index() {
        let node = Node::new(NodeId::new(0));

        let id = Id { index: vec![4], node: NodeId::new(0) };
        let new_id = node.truncate_and_replace_index(&id, 1, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.index.len() == 1);
        assert!(new_id.index[0] == 0);

        let id = Id { index: vec![4, 5, 3, 2], node: NodeId::new(0) };
        let new_id = node.truncate_and_replace_index(&id, 2, 0);
        assert!(new_id.node == NodeId::new(0));
        assert!(new_id.index.len() == 2);
        assert!(new_id.index[0] == 4);
        assert!(new_id.index[1] == 0);
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
            let a = node.id(&node.begin(), &node.end());
            let b = node.id(&a, &node.end());

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
                let new = node.id(id_0, id_1);
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
                let new = node.id(&first, &prev);
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
                let new = node.id(&prev, &last);
                assert!(&new != &prev);
                assert!(&new != &last);
                assert!(&new > &prev);
                assert!(&new < &last);
                prev = new;
            }
        }
    }
}
