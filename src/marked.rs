use std::collections::HashMap;
use std::ops::Neg;

use crate::hex::{Hex, DIRECTIONS};

/// A hexagonal tile whose six edges are each labeled with a value of type `L`.
///
/// Edge `i` faces the neighbor reachable via [`DIRECTIONS`]`[i]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedTile<L> {
    pub edges: [L; 6],
}

impl<L> MarkedTile<L> {
    pub const fn new(edges: [L; 6]) -> Self {
        Self { edges }
    }
}

impl<L: Copy> MarkedTile<L> {
    /// Rotate the tile CCW by `n` clicks (each click is 60°).
    ///
    /// After rotating CCW by `n`, the edge that was at position `i` faces
    /// direction `(i + n) % 6`, so `new[i] = old[(i + 6 - n % 6) % 6]`.
    pub fn rotate(&self, n: usize) -> Self {
        let n = n % 6;
        MarkedTile::new(std::array::from_fn(|i| self.edges[(i + 6 - n) % 6]))
    }

    /// Rotate CCW by one click (60°).
    pub fn rotate_ccw(&self) -> Self {
        self.rotate(1)
    }

    /// Rotate CW by one click (60°), equivalent to five CCW clicks.
    pub fn rotate_cw(&self) -> Self {
        self.rotate(5)
    }
}

/// An assignment of [`MarkedTile`]s to hex positions.
///
/// A tiling is **valid** when every pair of adjacent tiles has matching edge
/// labels: if tile `a` labels the shared edge `x` and tile `b` labels the same
/// edge `y` (from its own perspective), then `x == -y`.
pub struct MarkedTiling<L> {
    pub tiles: HashMap<Hex, MarkedTile<L>>,
}

impl<L> MarkedTiling<L> {
    pub fn new() -> Self {
        Self {
            tiles: HashMap::new(),
        }
    }

    pub fn insert(&mut self, hex: Hex, tile: MarkedTile<L>) {
        self.tiles.insert(hex, tile);
    }
}

impl<L> Default for MarkedTiling<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Clone + Neg<Output = L> + PartialEq> MarkedTiling<L> {
    /// Returns `true` iff every adjacent tile pair satisfies `x == -y` on
    /// their shared edge, where `x` and `y` are the labels seen from each
    /// tile respectively.
    pub fn is_valid(&self) -> bool {
        for (&hex, tile) in &self.tiles {
            for (i, &dir) in DIRECTIONS.iter().enumerate() {
                if let Some(neighbor) = self.tiles.get(&(hex + dir)) {
                    let opp = (i + 3) % 6;
                    if tile.edges[i] != -neighbor.edges[opp].clone() {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple signed-integer labels: negation is arithmetic negation.
    fn tile(edges: [i32; 6]) -> MarkedTile<i32> {
        MarkedTile::new(edges)
    }

    #[test]
    fn single_tile_always_valid() {
        let mut t = MarkedTiling::new();
        t.insert(Hex::new(0, 0), tile([1, 2, 3, 4, 5, 6]));
        assert!(t.is_valid());
    }

    #[test]
    fn two_tiles_valid() {
        // Tiles at (0,0) and (1,0): they share edge 0 / edge 3.
        // Edge 0 of (0,0) must equal -(edge 3 of (1,0)).
        let mut t = MarkedTiling::new();
        t.insert(Hex::new(0, 0), tile([7, 0, 0, 0, 0, 0]));
        t.insert(Hex::new(1, 0), tile([0, 0, 0, -7, 0, 0]));
        assert!(t.is_valid());
    }

    #[test]
    fn two_tiles_invalid() {
        let mut t = MarkedTiling::new();
        t.insert(Hex::new(0, 0), tile([7, 0, 0, 0, 0, 0]));
        t.insert(Hex::new(1, 0), tile([0, 0, 0, 7, 0, 0])); // should be -7
        assert!(!t.is_valid());
    }

    #[test]
    fn ring_valid() {
        // Build a ring of 7 tiles (center + 6 neighbors) with consistent labels.
        // Each spoke: center labels edge i with +1, neighbor labels opposite edge with -1.
        let mut t = MarkedTiling::new();
        let center = Hex::new(0, 0);
        t.insert(center, tile([1, 1, 1, 1, 1, 1]));
        for (i, &dir) in DIRECTIONS.iter().enumerate() {
            let opp = (i + 3) % 6;
            let mut edges = [0i32; 6];
            edges[opp] = -1;
            t.insert(center + dir, MarkedTile::new(edges));
        }
        assert!(t.is_valid());
    }

    #[test]
    fn zero_label_is_self_consistent() {
        // An edge labeled 0 requires the opposite to be 0 as well (-0 == 0).
        let mut t = MarkedTiling::new();
        t.insert(Hex::new(0, 0), tile([0; 6]));
        t.insert(Hex::new(1, 0), tile([0; 6]));
        assert!(t.is_valid());
    }

    #[test]
    fn rotate_zero_is_identity() {
        let t = tile([1, 2, 3, 4, 5, 6]);
        assert_eq!(t.rotate(0).edges, t.edges);
        assert_eq!(t.rotate(6).edges, t.edges);
    }

    #[test]
    fn rotate_ccw_shifts_edges() {
        let t = tile([1, 2, 3, 4, 5, 6]);
        let r = t.rotate_ccw();
        // CCW rotation: old edge i moves to position (i+1)%6, so new[i] = old[(i+5)%6].
        for i in 0..6 {
            assert_eq!(r.edges[i], t.edges[(i + 5) % 6]);
        }
    }

    #[test]
    fn rotate_cw_shifts_edges() {
        let t = tile([1, 2, 3, 4, 5, 6]);
        let r = t.rotate_cw();
        // CW rotation: new[i] = old[(i+1)%6].
        for i in 0..6 {
            assert_eq!(r.edges[i], t.edges[(i + 1) % 6]);
        }
    }

    #[test]
    fn rotate_ccw_six_times_is_identity() {
        let t = tile([1, 2, 3, 4, 5, 6]);
        let mut r = t.clone();
        for _ in 0..6 {
            r = r.rotate_ccw();
        }
        assert_eq!(r.edges, t.edges);
    }

    #[test]
    fn rotate_cw_and_ccw_are_inverses() {
        let t = tile([1, 2, 3, 4, 5, 6]);
        assert_eq!(t.rotate_ccw().rotate_cw().edges, t.edges);
        assert_eq!(t.rotate_cw().rotate_ccw().edges, t.edges);
    }
}
