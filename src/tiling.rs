use std::collections::HashMap;

use crate::hex::{Hex, DIRECTIONS};
use crate::marked::{MarkedTile, MarkedTiling};
use crate::spectre::{Label, DELTA, GAMMA, LAMBDA, PHI, PI, PSI, SIGMA, THETA, XI};

pub const BASE_TILES: [MarkedTile<Label>; 9] =
    [GAMMA, DELTA, THETA, LAMBDA, XI, PI, SIGMA, PHI, PSI];

pub const TILE_NAMES: [&str; 9] =
    ["Γ", "Δ", "Θ", "Λ", "Ξ", "Π", "Σ", "Φ", "Ψ"];

/// Returns `(type_index, rotation)` for a known tile, or `None`.
pub fn tile_id(tile: &MarkedTile<Label>) -> Option<(usize, usize)> {
    for (ti, base) in BASE_TILES.iter().enumerate() {
        for rot in 0..6 {
            if tile.edges == base.rotate(rot).edges {
                return Some((ti, rot));
            }
        }
    }
    None
}

fn is_compatible(tiling: &MarkedTiling<Label>, pos: Hex, candidate: &MarkedTile<Label>) -> bool {
    for (i, &dir) in DIRECTIONS.iter().enumerate() {
        if let Some(neighbor) = tiling.tiles.get(&(pos + dir)) {
            let opp = (i + 3) % 6;
            if candidate.edges[i] != -neighbor.edges[opp] {
                return false;
            }
        }
    }
    true
}

/// Returns `true` when there is at least one compatible tile for every frontier
/// position bordering `tiling` (i.e., the patch can be extended in all
/// directions without a dead end at distance 1).
pub fn frontier_is_extensible(tiling: &MarkedTiling<Label>) -> bool {
    for &pos in tiling.tiles.keys() {
        for &dir in &DIRECTIONS {
            let nb = pos + dir;
            if tiling.tiles.contains_key(&nb) {
                continue;
            }
            if compatible_ids(tiling, nb).is_empty() {
                return false;
            }
        }
    }
    true
}

/// All `(type_idx, rotation)` pairs compatible with every existing neighbor at `pos`.
pub fn compatible_ids(tiling: &MarkedTiling<Label>, pos: Hex) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (ti, base) in BASE_TILES.iter().enumerate() {
        for rot in 0..6 {
            let candidate = base.rotate(rot);
            if is_compatible(tiling, pos, &candidate) {
                out.push((ti, rot));
            }
        }
    }
    out
}

/// Generate a valid tiling patch by greedy BFS from a GAMMA seed.
///
/// Processes frontier positions ordered by the number of already-filled
/// neighbours (most constrained first), placing the first compatible tile.
/// Stops when `target_size` tiles have been placed or the frontier is empty.
pub fn generate_patch(target_size: usize) -> MarkedTiling<Label> {
    let mut tiling = MarkedTiling::new();
    let origin = Hex::new(0, 0);
    tiling.insert(origin, GAMMA);

    // frontier: pos → number of filled neighbours
    let mut frontier: HashMap<Hex, usize> = HashMap::new();
    for &dir in &DIRECTIONS {
        frontier.insert(origin + dir, 1);
    }

    while tiling.tiles.len() < target_size && !frontier.is_empty() {
        // Pick the most-constrained empty position.
        let pos = *frontier
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(p, _)| p)
            .unwrap();
        frontier.remove(&pos);

        // Forward-checking greedy: commit the first option that leaves every
        // immediate neighbour with at least one remaining compatible tile.
        let mut placed = false;
        'search: for base in BASE_TILES.iter() {
            for rot in 0..6_usize {
                let candidate = base.rotate(rot);
                if !is_compatible(&tiling, pos, &candidate) {
                    continue;
                }
                tiling.insert(pos, candidate);

                // Verify no immediate neighbour is now stranded.
                let mut safe = true;
                for &dir in &DIRECTIONS {
                    let nb = pos + dir;
                    if !tiling.tiles.contains_key(&nb) {
                        if compatible_ids(&tiling, nb).is_empty() {
                            safe = false;
                            break;
                        }
                    }
                }

                if safe {
                    placed = true;
                    break 'search;
                }
                // This option creates a dead end — undo and try the next.
                tiling.tiles.remove(&pos);
            }
        }

        if placed {
            // Add or update neighbours in the frontier.
            for &dir in &DIRECTIONS {
                let nb = pos + dir;
                if !tiling.tiles.contains_key(&nb) {
                    let filled = DIRECTIONS
                        .iter()
                        .filter(|&&d| tiling.tiles.contains_key(&(nb + d)))
                        .count();
                    frontier.insert(nb, filled);
                }
            }
        }
        // If still nothing fits (all options create a dead end) skip this
        // position — the patch remains valid but may have a gap here.
    }

    tiling
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_50_is_valid() {
        let t = generate_patch(50);
        assert_eq!(t.tiles.len(), 50);
        assert!(t.is_valid(), "50-tile patch failed edge-label validation");
    }

    #[test]
    fn patch_200_is_valid() {
        let t = generate_patch(200);
        assert_eq!(t.tiles.len(), 200);
        assert!(t.is_valid(), "200-tile patch failed edge-label validation");
    }

    #[test]
    fn patch_frontier_extensible() {
        // Every empty cell bordering the 100-tile patch must have at least one
        // compatible tile — the patch has no dead-end edges.
        let t = generate_patch(100);
        assert!(
            frontier_is_extensible(&t),
            "at least one frontier cell has no compatible tile"
        );
    }

    #[test]
    fn all_tile_ids_round_trip() {
        // tile_id is the inverse of (BASE_TILES[ti].rotate(rot)).
        for (i, base) in BASE_TILES.iter().enumerate() {
            for rot in 0..6 {
                let (ti, r) = tile_id(&base.rotate(rot))
                    .expect("known tile not recognized");
                assert_eq!(ti, i);
                assert_eq!(r, rot);
            }
        }
    }

    #[test]
    fn gamma_east_neighbor_is_sigma_rot1() {
        // The East neighbor of GAMMA(rot=0) is uniquely forced to SIGMA(rot=1).
        let mut t = MarkedTiling::new();
        t.insert(Hex::new(0, 0), GAMMA);
        let east = Hex::new(0, 0) + DIRECTIONS[0];
        let ids = compatible_ids(&t, east);
        assert_eq!(ids.len(), 1, "expected exactly one compatible E-neighbor");
        assert_eq!(ids[0], (6, 1), "E-neighbor should be SIGMA(rot=1) [index 6]");
    }
}
