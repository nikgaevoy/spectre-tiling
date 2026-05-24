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
    pub fn new(edges: [L; 6]) -> Self {
        Self { edges }
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
}
