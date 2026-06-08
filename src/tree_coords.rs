use crate::hex::Hex;

/// One child tile inside a base supertile, expressed in the supertile's local
/// frame. `type_idx` indexes `BASE_TILES` / `TILE_NAMES` and `rotation` is the
/// CCW rotation (0–5) applied to that base tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Child {
    pub hex: Hex,
    pub type_idx: u8,
    pub rotation: u8,
}

const fn c(q: i32, r: i32, type_idx: u8, rotation: u8) -> Child {
    Child { hex: Hex::new(q, r), type_idx, rotation }
}

/// Children of each base supertile, ordered along the supertile's unique
/// Hamiltonian cycle on hex-cell adjacency, starting at the root tile
/// (GAMMA at local (0,0)) and traversing CCW (first step to (0,1) = NE).
///
/// Outer index: supertile type. Order matches `BASE_TILES` / `TILE_NAMES`:
/// Γ=0 Δ=1 Θ=2 Λ=3 Ξ=4 Π=5 Σ=6 Φ=7 Ψ=8. Γ has 7 children; the rest have 8.
///
/// The cycle's position sequence is identical for all 8-tile supertiles
/// (the underlying hex graph is the same); only the tile types and rotations
/// at each position differ. Γ omits the (2,-2) cell.
pub const SUPERTILE_CHILDREN: [&[Child]; 9] = [
    // Γ — 7 tiles, no (2,-2)
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 4, 1), // XI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 2, 0), // THETA
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 5, 4), // PI.rotate(4)
    ],
    // Δ
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 5, 1), // PI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 4, 5), // XI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 4, 4), // XI.rotate(4)
    ],
    // Θ
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 5, 1), // PI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 5, 5), // PI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 8, 4), // PSI.rotate(4)
    ],
    // Λ
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 5, 1), // PI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 4, 5), // XI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 8, 4), // PSI.rotate(4)
    ],
    // Ξ
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 8, 1), // PSI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 5, 5), // PI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 8, 4), // PSI.rotate(4)
    ],
    // Π
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 8, 1), // PSI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 4, 5), // XI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 8, 4), // PSI.rotate(4)
    ],
    // Σ
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 3, 2), // LAMBDA.rotate(2)
        c(1,  1, 5, 1), // PI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 4, 5), // XI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 4, 4), // XI.rotate(4)
    ],
    // Φ
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 5, 1), // PI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 8, 5), // PSI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 8, 4), // PSI.rotate(4)
    ],
    // Ψ
    &[
        c(0,  0, 0, 0), // GAMMA
        c(0,  1, 7, 2), // PHI.rotate(2)
        c(1,  1, 8, 1), // PSI.rotate(1)
        c(1,  0, 6, 1), // SIGMA.rotate(1)
        c(2, -1, 7, 0), // PHI
        c(2, -2, 8, 5), // PSI.rotate(5)
        c(1, -1, 1, 5), // DELTA.rotate(5)
        c(0, -1, 8, 4), // PSI.rotate(4)
    ],
];

/// Path through the supertile substitution hierarchy. Each entry indexes into
/// `SUPERTILE_CHILDREN` of the parent supertile (which is determined by the
/// child picked at the previous level). The supertile type at the top of the
/// hierarchy is external context, not stored here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TreeCoords {
    pub path: Vec<u8>,
}

impl TreeCoords {
    pub fn new() -> Self {
        Self { path: Vec::new() }
    }

    pub fn depth(&self) -> usize {
        self.path.len()
    }

    pub fn push(&mut self, child: u8) {
        self.path.push(child);
    }

    pub fn pop(&mut self) -> Option<u8> {
        self.path.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::DIRECTIONS;
    use crate::supertile::BASE_SUPERTILE_FNS;
    use crate::tiling::{tile_id, BASE_TILES};

    /// Every consecutive pair in the cycle (including last→first) must be hex-adjacent.
    #[test]
    fn children_form_a_cycle() {
        for (ti, children) in SUPERTILE_CHILDREN.iter().enumerate() {
            for i in 0..children.len() {
                let a = children[i].hex;
                let b = children[(i + 1) % children.len()].hex;
                let diff = b - a;
                assert!(
                    DIRECTIONS.contains(&diff),
                    "supertile {} children {} -> {} not adjacent ({:?})",
                    ti, i, (i + 1) % children.len(), diff
                );
            }
        }
    }

    /// Each supertile starts with GAMMA at (0,0) and its first step is NE = (0,1).
    #[test]
    fn cycle_starts_at_root_going_ne() {
        for children in SUPERTILE_CHILDREN.iter() {
            assert_eq!(children[0].hex, Hex::new(0, 0));
            assert_eq!(children[0].type_idx, 0);
            assert_eq!(children[0].rotation, 0);
            assert_eq!(children[1].hex, Hex::new(0, 1));
        }
    }

    /// SUPERTILE_CHILDREN agrees with the supertiles produced by BASE_SUPERTILE_FNS:
    /// same set of positions and same (type, rotation) at each position.
    #[test]
    fn children_match_supertile_definitions() {
        for (ti, children) in SUPERTILE_CHILDREN.iter().enumerate() {
            let tiling = BASE_SUPERTILE_FNS[ti]();
            assert_eq!(tiling.tiles.len(), children.len(), "supertile {} size", ti);
            for child in children.iter() {
                let actual = tiling.tiles.get(&child.hex)
                    .unwrap_or_else(|| panic!("supertile {} missing tile at {:?}", ti, child.hex));
                let expected = BASE_TILES[child.type_idx as usize].rotate(child.rotation as usize);
                assert_eq!(
                    actual.edges, expected.edges,
                    "supertile {} at {:?}: type/rotation mismatch (table says {:?})",
                    ti, child.hex, tile_id(actual)
                );
            }
        }
    }
}
