use std::collections::{HashMap, HashSet, VecDeque};

use crate::hex::{Hex, DIRECTIONS};
use crate::marked::{MarkedTile, MarkedTiling};
use crate::spectre::{Label, DELTA, GAMMA, LAMBDA, PHI, PI, PSI, SIGMA, THETA, XI};
use crate::supertile::{AnchorPoint, BASE_SUPERTILE_FNS, SUPERTILE_ANCHORS};

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
    tiling
        .tiles
        .keys()
        .flat_map(|&pos| DIRECTIONS.iter().map(move |&dir| pos + dir))
        .filter(|nb| !tiling.tiles.contains_key(nb))
        .all(|nb| !compatible_ids(tiling, nb).is_empty())
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
        let pos = *frontier.iter().max_by_key(|&(_, &c)| c).unwrap().0;
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

/// Returns the two indices into `SUPERTILE_ANCHORS[type]` for the original-tile
/// corners bounding the edge at world-direction `edge_dir`, given the supertile
/// is CCW-rotated by `rot` steps.  The two indices are in screen-CW order.
///
/// `SUPERTILE_ANCHORS[type][i]` enumerates the un-rotated original tile's
/// corners CCW starting from N: 0=N, 1=WNW, 2=WSW, 3=S, 4=ESE, 5=ENE.
fn edge_corners(edge_dir: usize, rot: usize) -> [usize; 2] {
    let rot = rot % 6;
    [(edge_dir + 11 - rot) % 6, (edge_dir + 10 - rot) % 6]
}

/// Returns the hex offset of the `to` supertile origin relative to the `from`
/// supertile origin, given they are adjacent in the original tiling with
/// `from`'s edge `edge_dir` facing `to`.
///
/// `from_type` / `to_type` are indices into `BASE_TILES` / `TILE_NAMES` (Γ=0 … Ψ=8).
/// `from_rot` / `to_rot` are CCW rotation counts (0–5).
pub fn infer_supertile_offset(
    from_type: usize,
    from_rot: usize,
    edge_dir: usize,
    to_type: usize,
    to_rot: usize,
) -> (usize, Hex) {
    let [f1, f2] = edge_corners(edge_dir, from_rot);
    let [t1, t2] = edge_corners((edge_dir + 3) % 6, to_rot);

    let v_f1 = anchor_vertex(SUPERTILE_ANCHORS[from_type][f1].rotate(from_rot));
    let v_f2 = anchor_vertex(SUPERTILE_ANCHORS[from_type][f2].rotate(from_rot));
    let v_t1 = anchor_vertex(SUPERTILE_ANCHORS[to_type][t1].rotate(to_rot));
    let v_t2 = anchor_vertex(SUPERTILE_ANCHORS[to_type][t2].rotate(to_rot));

    // Along the shared edge: f1→f2 (CW around `from`) and t1→t2 (CW around `to`)
    // traverse it in opposite directions, so once `to` is placed the world
    // vector F1−F2 must equal T2−T1.  If those vectors differ only by a 60°
    // rotation, that rotation is the extra spin `to` needs.  Only when no such
    // 60° rotation exists (e.g. the edge lengths disagree) does no answer
    // exist; that is the assertion case.
    let ctx = || format!(
        "from={}[{from_type}] rot={from_rot}, edge_dir={edge_dir}, \
         to={}[{to_type}] rot={to_rot}",
        TILE_NAMES[from_type], TILE_NAMES[to_type],
    );

    let f_vec = (v_f1.0 - v_f2.0, v_f1.1 - v_f2.1);
    let mut tv = (v_t2.0 - v_t1.0, v_t2.1 - v_t1.1);
    let extra = (0..6)
        .find(|_| {
            if tv == f_vec {
                true
            } else {
                tv = rotate_vertex_step(tv);
                false
            }
        })
        .unwrap_or_else(|| panic!(
            "no 60° rotation aligns the t-anchor edge with the f-anchor edge ({})",
            ctx(),
        ));

    // World position of t2 after the extra rotation; offset = F1 − rotated T2.
    let mut t2 = v_t2;
    for _ in 0..extra {
        t2 = rotate_vertex_step(t2);
    }
    let dx = v_f1.0 - t2.0;
    let dy = v_f1.1 - t2.1;
    assert!(dy % 3 == 0, "offset is not a hex lattice translation ({})", ctx());
    let oy = -dy / 3;
    let two_ox = dx - oy;
    assert!(two_ox % 2 == 0, "offset is not a hex lattice translation ({})", ctx());

    ((to_rot + extra) % 6, Hex::new(two_ox / 2, oy))
}

/// One CCW 60° step of the supertile rotation, applied to scaled vertex coords.
/// Mirrors `AnchorPoint::rotate(1)` on the vertex-coord side: a vertex at
/// `(X, Y)` maps to `((X + Y) / 2, (Y − 3·X) / 2)` (both numerators are even —
/// see `DX[c] + DY[c]` is always even — so the division is exact).
fn rotate_vertex_step((x, y): (i32, i32)) -> (i32, i32) {
    ((x + y) / 2, (y - 3 * x) / 2)
}

/// Exact integer 2D coordinates of the vertex described by `ap`.  Uses the
/// pointy-top hex layout with √3 dropped from x and both axes doubled, so the
/// hex grid sits on Z².  A hex translation by (ox, oy) shifts these coords by
/// (2·ox + oy, -3·oy).
fn anchor_vertex(ap: AnchorPoint) -> (i32, i32) {
    const DX: [i32; 6] = [1, 0, -1, -1, 0, 1];
    const DY: [i32; 6] = [1, 2, 1, -1, -2, -1];
    let c = ap.corner as usize;
    (2 * ap.hex.q + ap.hex.r + DX[c], -3 * ap.hex.r + DY[c])
}

/// Replace each tile in `tiling` with the corresponding supertile, stitched
/// together via BFS.  Returns the new tiling and, for each original tile, the
/// set of hex positions it expanded into (one `HashSet<Hex>` per supertile).
pub fn supersubstitute_with_regions(
    tiling: &MarkedTiling<Label>,
) -> (MarkedTiling<Label>, Vec<HashSet<Hex>>) {
    let id_at = |pos: &Hex| {
        tile_id(&tiling.tiles[pos]).expect("unrecognized tile in supersubstitute")
    };

    let mut known: HashMap<Hex, (usize, usize, Hex)> = HashMap::new();
    let mut queue: VecDeque<Hex> = VecDeque::new();

    let Some(&start) = tiling.tiles.keys().min_by_key(|h| (h.q, h.r)) else {
        return (MarkedTiling::new(), Vec::new());
    };
    let (t0, r0) = id_at(&start);
    known.insert(start, (t0, r0, Hex::new(0, 0)));
    queue.push_back(start);

    while let Some(pos_a) = queue.pop_front() {
        let (type_a, super_rot_a, offset_a) = known[&pos_a];
        let (_, tile_rot_a) = id_at(&pos_a);
        // `infer_supertile_offset` computes its answer in a frame where A sits
        // at origin with rotation `tile_rot_a` — but in the world A is at
        // `offset_a` with rotation `super_rot_a`.  Convert the returned
        // (super_rot, delta) by rotating both by the difference.
        let delta_rot = (super_rot_a + 6 - tile_rot_a) % 6;
        for (d, &dir) in DIRECTIONS.iter().enumerate() {
            let pos_b = pos_a + dir;
            if tiling.tiles.contains_key(&pos_b) && !known.contains_key(&pos_b) {
                let (type_b, tile_rot_b) = id_at(&pos_b);
                let (sr_b_fn, mut delta) =
                    infer_supertile_offset(type_a, tile_rot_a, d, type_b, tile_rot_b);
                for _ in 0..delta_rot {
                    delta = delta.rotate_cw();
                }
                let super_rot_b = (sr_b_fn + delta_rot) % 6;
                known.insert(pos_b, (type_b, super_rot_b, offset_a + delta));
                queue.push_back(pos_b);
            }
        }
    }

    let mut result = MarkedTiling::new();
    let mut regions = Vec::new();
    for (_, (type_idx, rot, offset)) in known {
        let mut region = HashSet::new();
        for (local_hex, tile) in BASE_SUPERTILE_FNS[type_idx]().rotate(rot).tiles {
            let world_hex = local_hex + offset;
            result.insert(world_hex, tile);
            region.insert(world_hex);
        }
        regions.push(region);
    }
    (result, regions)
}

/// Replace each tile in `tiling` with the corresponding supertile, stitched
/// together via BFS.  Positions of individual supertiles are determined by
/// calling [`infer_supertile_offset`] for each adjacent pair.
pub fn supersubstitute(tiling: &MarkedTiling<Label>) -> MarkedTiling<Label> {
    supersubstitute_with_regions(tiling).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn normalize(t: &MarkedTiling<Label>) -> BTreeMap<(i32, i32), [i8; 6]> {
        let &min_pos = t.tiles.keys().min_by_key(|h| (h.q, h.r)).unwrap();
        t.tiles
            .iter()
            .map(|(&h, tile)| {
                let p = h - min_pos;
                ((p.q, p.r), tile.edges.map(|e| e as i8))
            })
            .collect()
    }

    /// Stable 64-bit hash of a tiling, invariant under hex translation and
    /// 60° rotation.  Canonical form is the lex-min serialization across the
    /// 6 rotations of `t`, each translated so its min-(q,r) hex sits at the
    /// origin.  Hashed with FNV-1a so the value is stable across Rust
    /// versions (unlike `std::hash::DefaultHasher`).
    fn canonical_hash(t: &MarkedTiling<Label>) -> u64 {
        let canonical = (0..6)
            .map(|n| {
                normalize(&t.rotate(n))
                    .into_iter()
                    .flat_map(|((q, r), edges)| {
                        let mut buf = Vec::with_capacity(4 + 4 + 6);
                        buf.extend_from_slice(&q.to_le_bytes());
                        buf.extend_from_slice(&r.to_le_bytes());
                        buf.extend(edges.iter().map(|&e| e as u8));
                        buf
                    })
                    .collect::<Vec<u8>>()
            })
            .min()
            .unwrap();

        let mut h: u64 = 0xcbf29ce484222325;
        for b in canonical {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

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

    #[test]
    fn supertile_gamma_anchor_edges_match_in_euclidean_length() {
        // For every adjacent (A, B) base-tile pair in supertile_gamma, the
        // anchor edge picked by `edge_corners` from A's side must have the
        // same Euclidean length² as the one picked from B's side.  Length²
        // in scaled vertex coords is 3·X² + Y² (since x_real = √3·X/2,
        // y_real = Y/2).  This is a precondition for `infer_supertile_offset`
        // to find a 60° rotation aligning the two edges.
        use crate::supertile::supertile_gamma;

        let eucl_sq = |(x, y): (i32, i32)| 3 * x * x + y * y;

        let t = supertile_gamma();
        for (&pa, tile_a) in &t.tiles {
            let (ta, ra) = tile_id(tile_a).unwrap();
            for d in 0..6 {
                let pb = pa + DIRECTIONS[d];
                let Some(tile_b) = t.tiles.get(&pb) else { continue };
                let (tb, rb) = tile_id(tile_b).unwrap();
                let [f1, f2] = edge_corners(d, ra);
                let [u1, u2] = edge_corners((d + 3) % 6, rb);
                let vf1 = anchor_vertex(SUPERTILE_ANCHORS[ta][f1].rotate(ra));
                let vf2 = anchor_vertex(SUPERTILE_ANCHORS[ta][f2].rotate(ra));
                let vt1 = anchor_vertex(SUPERTILE_ANCHORS[tb][u1].rotate(rb));
                let vt2 = anchor_vertex(SUPERTILE_ANCHORS[tb][u2].rotate(rb));
                let lf = eucl_sq((vf1.0 - vf2.0, vf1.1 - vf2.1));
                let lt = eucl_sq((vt2.0 - vt1.0, vt2.1 - vt1.1));
                assert_eq!(
                    lf, lt,
                    "anchor edge length mismatch: {}({}) at ({},{}) --d{}--> {}({}) at ({},{}) \
                     | anchors f[{},{}] t[{},{}] | Eucl² f={} t={}",
                    TILE_NAMES[ta], ra, pa.q, pa.r, d,
                    TILE_NAMES[tb], rb, pb.q, pb.r,
                    f1, f2, u1, u2, lf, lt,
                );
            }
        }
    }

    #[test]
    fn supersubstitute_gamma_result_is_connected() {
        // `supertile_gamma()` is a connected patch (every tile shares an edge
        // with at least one other).  The substitution replaces each tile with
        // a connected supertile and is supposed to stitch them together along
        // shared abstract edges, so the output must remain edge-connected.
        // A disconnected result means the BFS placed at least one supertile
        // detached from the rest — a wrong offset.
        use crate::supertile::supertile_gamma;
        use std::collections::VecDeque;

        let result = supersubstitute(&supertile_gamma());
        assert!(!result.tiles.is_empty());

        let start = *result.tiles.keys().next().unwrap();
        let mut seen: HashSet<Hex> = HashSet::new();
        seen.insert(start);
        let mut queue: VecDeque<Hex> = VecDeque::from([start]);
        while let Some(h) = queue.pop_front() {
            for &dir in &DIRECTIONS {
                let n = h + dir;
                if result.tiles.contains_key(&n) && seen.insert(n) {
                    queue.push_back(n);
                }
            }
        }

        if seen.len() != result.tiles.len() {
            let mut unreached: Vec<_> = result
                .tiles
                .keys()
                .filter(|h| !seen.contains(h))
                .collect();
            unreached.sort_by_key(|h| (h.q, h.r));
            panic!(
                "result is disconnected: reached {}/{} tiles from {:?}; \
                 unreached: {:?}",
                seen.len(),
                result.tiles.len(),
                start,
                unreached,
            );
        }
    }

    #[test]
    fn supersubstitute_commutes_with_input_shift_and_rotation() {
        // Any rigid-motion-equivalent input must produce a rigid-motion-equivalent
        // output.  Concretely: rotating then shifting `supertile_gamma()` and
        // running supersubstitute should give the same normalized result as
        // rotating the baseline supersubstitute output by the same amount.
        // Trying many shifts also stresses the BFS start-tile choice
        // (`HashMap::iter().next()`), which depends on hex coordinates.
        use crate::supertile::supertile_gamma;

        let base = supertile_gamma();
        let base_result = supersubstitute(&base);

        let shifts = [
            Hex::new(0, 0), Hex::new(1, 0), Hex::new(-3, 4), Hex::new(7, -2),
            Hex::new(0, -5), Hex::new(-10, 10), Hex::new(2, 3),
        ];

        let mut failures: Vec<(usize, Hex, usize, usize)> = Vec::new();
        for rot in 0..6 {
            let rotated_base = base.rotate(rot);
            // The BFS fixes an arbitrary global rotation via the start tile,
            // so two valid runs are equal up to rigid motion, not just
            // translation.  Compare against all 6 rotations of the baseline.
            let expected_orbit: [_; 6] = std::array::from_fn(|k| {
                normalize(&base_result.rotate((rot + k) % 6))
            });

            for &shift in &shifts {
                let mut input = MarkedTiling::new();
                for (&h, tile) in &rotated_base.tiles {
                    input.insert(h + shift, tile.clone());
                }
                let result = supersubstitute(&input);
                let actual = normalize(&result);
                if !expected_orbit.contains(&actual) {
                    failures.push((rot, shift, actual.len(), expected_orbit[0].len()));
                }
            }
        }

        if !failures.is_empty() {
            let n = failures.len();
            let total = 6 * shifts.len();
            let mut summary = String::new();
            for (rot, shift, got, want) in failures.iter().take(10) {
                use std::fmt::Write;
                let _ = writeln!(
                    summary,
                    "  rot={} shift=({},{}): got {} tiles, expected {}",
                    rot, shift.q, shift.r, got, want,
                );
            }
            panic!(
                "{}/{} (rot, shift) combinations are not equivariant:\n{}",
                n, total, summary
            );
        }
    }

    #[test]
    fn supersubstitute_is_rotation_equivariant_on_gamma() {
        // Rotating the input by n should produce a result equal to the
        // unrotated result rotated by n, up to a hex translation.  This is
        // a pure property of the algorithm: as long as anchor edges are
        // locally length-consistent, every individual `infer_supertile_offset`
        // call commutes with rotation, so the whole BFS should too.
        use crate::supertile::supertile_gamma;

        let base_input = supertile_gamma();
        let base_output = supersubstitute(&base_input);

        for n in 1..6 {
            let rotated_input = base_input.rotate(n);
            let rotated_output = supersubstitute(&rotated_input);
            let actual = normalize(&rotated_output);

            // BFS init fixes an arbitrary global rotation, so compare against
            // all 6 rotations of the baseline result.
            let expected_orbit: [_; 6] =
                std::array::from_fn(|k| normalize(&base_output.rotate((n + k) % 6)));

            assert_eq!(
                actual.len(), expected_orbit[0].len(),
                "n={}: tile-count differs ({} vs {})",
                n, actual.len(), expected_orbit[0].len(),
            );

            assert!(
                expected_orbit.contains(&actual),
                "n={}: result not rigid-motion-equivalent to baseline.rotate({})",
                n, n,
            );
        }
    }

    #[test]
    fn supersubstitute_each_supertile_is_valid() {
        // Seeding `supersubstitute` with any of the 9 base supertiles should
        // produce an edge-label-consistent patch.
        for (i, supertile_fn) in BASE_SUPERTILE_FNS.iter().enumerate() {
            let input = supertile_fn();
            let result = supersubstitute(&input);
            assert!(
                !result.tiles.is_empty(),
                "supersubstitute({}) produced an empty tiling",
                TILE_NAMES[i],
            );
            assert!(
                result.is_valid(),
                "supersubstitute({}) produced an invalid tiling",
                TILE_NAMES[i],
            );
        }
    }

    #[test]
    fn supersubstitute_each_rotated_supertile_is_valid() {
        // Validity should be invariant under rotation of the input — every
        // 60° rotation of every base supertile must substitute to a valid
        // patch.
        for (i, supertile_fn) in BASE_SUPERTILE_FNS.iter().enumerate() {
            let base = supertile_fn();
            for rot in 0..6 {
                let input = base.rotate(rot);
                let result = supersubstitute(&input);
                assert!(
                    result.is_valid(),
                    "supersubstitute({}.rotate({})) produced an invalid tiling",
                    TILE_NAMES[i], rot,
                );
            }
        }
    }

    #[test]
    fn supersubstitute_twice_each_rotated_supertile_is_valid() {
        // Iterating the substitution should preserve validity: applying
        // `supersubstitute` twice to any rotated base supertile must still
        // yield a valid tiling.
        for (i, supertile_fn) in BASE_SUPERTILE_FNS.iter().enumerate() {
            let base = supertile_fn();
            for rot in 0..6 {
                let input = base.rotate(rot);
                let once = supersubstitute(&input);
                let twice = supersubstitute(&once);
                assert!(
                    twice.is_valid(),
                    "supersubstitute(supersubstitute({}.rotate({}))) \
                     produced an invalid tiling",
                    TILE_NAMES[i], rot,
                );
            }
        }
    }

    #[test]
    fn supersubstitute_gamma_regions_disjoint() {
        use crate::supertile::supertile_gamma;

        let base = supertile_gamma();
        let base_count = base.tiles.len();
        let (result, regions) = supersubstitute_with_regions(&base);

        assert_eq!(regions.len(), base_count, "one region per base tile");

        let total_region_tiles: usize = regions.iter().map(|r| r.len()).sum();
        assert_eq!(
            result.tiles.len(),
            total_region_tiles,
            "result has {} tiles but regions sum to {} — some supertiles overlap",
            result.tiles.len(),
            total_region_tiles,
        );

        for i in 0..regions.len() {
            for j in (i + 1)..regions.len() {
                let overlap: Vec<_> = regions[i].intersection(&regions[j]).collect();
                assert!(
                    overlap.is_empty(),
                    "supertiles {} and {} overlap at {:?}",
                    i, j, overlap,
                );
            }
        }
    }

    #[test]
    fn canonical_hash_is_rigid_motion_invariant() {
        // Sanity-check `canonical_hash` itself before relying on it for the
        // 4-iteration regression test: rotating or shifting the input must
        // not change the hash.
        use crate::supertile::supertile_gamma;

        let base = supertile_gamma();
        let h0 = canonical_hash(&base);

        for n in 1..6 {
            assert_eq!(
                canonical_hash(&base.rotate(n)),
                h0,
                "rotation {n} changes canonical_hash",
            );
        }

        for shift in [Hex::new(3, -1), Hex::new(-5, 7), Hex::new(10, 0)] {
            let mut shifted = MarkedTiling::new();
            for (&h, tile) in &base.tiles {
                shifted.insert(h + shift, tile.clone());
            }
            assert_eq!(
                canonical_hash(&shifted),
                h0,
                "shift ({},{}) changes canonical_hash",
                shift.q, shift.r,
            );
        }
    }

    #[test]
    fn supersubstitute_four_iterations_on_gamma_has_expected_hash() {
        // Pin the exact output of supersubstitute^4 starting from a single
        // GAMMA tile.  The hash is canonical under hex translation and 60°
        // rotation, so it is robust to BFS start-tile choice (HashMap
        // iteration order) — see
        // `supersubstitute_commutes_with_input_shift_and_rotation` for why
        // those are the only sources of nondeterminism.
        let mut t = MarkedTiling::new();
        t.insert(Hex::new(0, 0), GAMMA);
        for _ in 0..4 {
            t = supersubstitute(&t);
        }
        assert!(t.is_valid(), "iterated supersubstitute produced an invalid tiling");

        const EXPECTED_TILES: usize = 3409;
        const EXPECTED_HASH: u64 = 4960813579813931908;
        assert_eq!(
            t.tiles.len(),
            EXPECTED_TILES,
            "supersubstitute^4(GAMMA) tile count drifted",
        );
        assert_eq!(
            canonical_hash(&t),
            EXPECTED_HASH,
            "supersubstitute^4(GAMMA) hash changed — output content drifted",
        );
    }
}
