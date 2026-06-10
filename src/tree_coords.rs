use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::OnceLock;

use crate::hex::{Hex, DIRECTIONS};
use crate::marked::MarkedTiling;
use crate::spectre::Label;
use crate::supertile::{AnchorPoint, SUPERTILE_ANCHORS};
use crate::tiling::{anchor_vertex, canonical_supersubstitute_with_placements, BASE_TILES};

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

/// Child index of the cell at `hex` within base supertile `p`, if any.
pub fn child_at(p: usize, hex: Hex) -> Option<u8> {
    SUPERTILE_CHILDREN[p]
        .iter()
        .position(|c| c.hex == hex)
        .map(|i| i as u8)
}

/// Supertile types along `path` under top-level type `top`: `out[0] = top`,
/// `out[k+1]` = type of the child selected by `path[k]` (so the last entry is
/// the leaf tile's type).
pub fn types_along(top: u8, path: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(path.len() + 1);
    out.push(top);
    for &c in path {
        let t = *out.last().unwrap() as usize;
        out.push(SUPERTILE_CHILDREN[t][c as usize].type_idx);
    }
    out
}

/// CCW rotation of the leaf tile relative to the top supertile's frame.
///
/// The substitution is orientation-reversing: a tile with rotation ρ expands
/// to its supertile patch rotated by −ρ (each level is a mirrored realization
/// of the previous one).  Child rotations therefore compose by alternating
/// sum, not by plain addition: ρ ← r_child − ρ at each level going down.
pub fn path_rotation(top: u8, path: &[u8]) -> u8 {
    let mut t = top as usize;
    let mut rot = 0;
    for &c in path {
        let ch = &SUPERTILE_CHILDREN[t][c as usize];
        rot = (ch.rotation as usize + 6 - rot) % 6;
        t = ch.type_idx as usize;
    }
    rot as u8
}

/// One hex-edge segment on a supertile patch boundary: the side of cell
/// `child` facing direction `dir` (both in the supertile's base frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeSeg {
    pub child: u8,
    pub dir: u8,
}

/// Boundary decomposition of each base supertile patch.
///
/// The patch boundary is a single cycle of hex edges, walked with the patch
/// interior on a consistent side.  The 6 anchor vertices
/// (`SUPERTILE_ANCHORS`) split the cycle into 6 *super-edges*; super-edge `E`
/// runs from anchor `(E+4)%6` to anchor `(E+5)%6` and plays the role that
/// hex-edge direction `E` plays for a single tile.  Because both patches of
/// a glued super-edge pair walk their boundary with the same winding, the
/// shared path is traversed in opposite orders: segment `k` of one edge is
/// glued to segment `len-1-k` of the other.
pub struct BoundaryTables {
    /// `edge_seq[p][e]` = segments of super-edge `e` of supertile type `p`,
    /// in boundary-walk order.
    pub edge_seq: [[Vec<EdgeSeg>; 6]; 9],
    /// Inverse: `(child, dir)` → `(e, k)` with `edge_seq[p][e][k] == (child, dir)`.
    pub seg_lookup: [HashMap<(u8, u8), (u8, u8)>; 9],
}

/// Boundary tables for all 9 base supertiles (built once, on first use).
pub fn boundary_tables() -> &'static BoundaryTables {
    static TABLES: OnceLock<BoundaryTables> = OnceLock::new();
    TABLES.get_or_init(build_boundary_tables)
}

/// Hex-edge `(h, d)` endpoints are corners `(6-d)%6` and `(5-d)%6` of `h`
/// (in `hex_app::corner` numbering); directing every boundary edge this way
/// gives a consistent winding, so each boundary vertex starts exactly one
/// directed segment and the walk is well defined.
fn corner_vertex(hex: Hex, corner: usize) -> (i32, i32) {
    anchor_vertex(AnchorPoint::new(hex, corner as u8))
}

fn build_boundary_tables() -> BoundaryTables {
    let mut edge_seq: [[Vec<EdgeSeg>; 6]; 9] = Default::default();
    let mut seg_lookup: [HashMap<(u8, u8), (u8, u8)>; 9] = Default::default();

    for p in 0..9 {
        let children = SUPERTILE_CHILDREN[p];

        // Directed boundary segments, keyed by start vertex.
        let mut by_start: HashMap<(i32, i32), EdgeSeg> = HashMap::new();
        for (i, c) in children.iter().enumerate() {
            for (d, &dir) in DIRECTIONS.iter().enumerate() {
                if child_at(p, c.hex + dir).is_none() {
                    let start = corner_vertex(c.hex, (6 - d) % 6);
                    let prev = by_start.insert(
                        start,
                        EdgeSeg { child: i as u8, dir: d as u8 },
                    );
                    assert!(prev.is_none(), "type {p}: boundary pinched at {start:?}");
                }
            }
        }

        // Walk the boundary cycle.
        let total = by_start.len();
        let first = *by_start.keys().min().unwrap();
        let mut walk: Vec<EdgeSeg> = Vec::with_capacity(total);
        let mut starts: Vec<(i32, i32)> = Vec::with_capacity(total);
        let mut v = first;
        loop {
            let seg = by_start[&v];
            walk.push(seg);
            starts.push(v);
            v = corner_vertex(
                children[seg.child as usize].hex,
                (5 - seg.dir as usize) % 6,
            );
            if v == first {
                break;
            }
        }
        assert_eq!(walk.len(), total, "type {p}: boundary is not a single cycle");

        // Locate the 6 anchors on the walk.  They must appear in a consistent
        // cyclic order; because the substitution is orientation-reversing
        // (each patch is a mirrored realization of its abstract tile), the
        // anchor indices run *descending* along our walk, opposite to the
        // single-hex case.  Detect the direction and assert consistency.
        let pos: [usize; 6] = std::array::from_fn(|ai| {
            let va = anchor_vertex(SUPERTILE_ANCHORS[p][ai]);
            starts
                .iter()
                .position(|&s| s == va)
                .unwrap_or_else(|| panic!("type {p}: anchor {ai} not on boundary"))
        });
        let span = |from: usize, to: usize| (to + total - from) % total;
        let descending = span(pos[0], pos[5]) < span(pos[0], pos[1]);
        for ai in 0..6 {
            let next = if descending { (ai + 5) % 6 } else { (ai + 1) % 6 };
            let skip = if descending { (ai + 4) % 6 } else { (ai + 2) % 6 };
            assert!(
                span(pos[ai], pos[next]) <= span(pos[ai], pos[skip]),
                "type {p}: anchors out of cyclic order on boundary walk",
            );
        }

        // Split the walk into the 6 super-edges: super-edge `e` is the
        // boundary path between anchors (e+4)%6 and (e+5)%6, taken in walk
        // order.
        for (e, seq) in edge_seq[p].iter_mut().enumerate() {
            let (a, b) = ((e + 4) % 6, (e + 5) % 6);
            let (from, to) = if descending {
                (pos[b], pos[a])
            } else {
                (pos[a], pos[b])
            };
            let mut k = from;
            while k != to {
                let seg = walk[k];
                seg_lookup[p].insert((seg.child, seg.dir), (e as u8, seq.len() as u8));
                seq.push(seg);
                k = (k + 1) % total;
            }
            assert!(!seq.is_empty(), "type {p}: empty super-edge {e}");
        }
        assert_eq!(seg_lookup[p].len(), total);
    }

    BoundaryTables { edge_seq, seg_lookup }
}

/// Symmetric relation over `(supertile type, super-edge)` pairs: which
/// super-edges can ever abut in a tiling generated by the substitution.
///
/// Seeded with sibling adjacencies inside each base supertile and closed
/// under descent: if `(a, ea)` and `(b, eb)` are glued, segment `k` of one
/// is glued to segment `len-1-k` of the other, yielding a child-level pair.
/// The closure asserts that glued super-edges always have equally many
/// segments — Tatham's completeness check for the substitution system.
pub fn edge_adjacency() -> &'static EdgeAdjacency {
    static REL: OnceLock<EdgeAdjacency> = OnceLock::new();
    REL.get_or_init(|| build_edge_adjacency(boundary_tables()))
}

/// `(type, super-edge)` → set of `(type, super-edge)` it can be glued to.
pub type EdgeAdjacency = HashMap<(u8, u8), BTreeSet<(u8, u8)>>;

fn build_edge_adjacency(bt: &BoundaryTables) -> EdgeAdjacency {
    let mut rel = EdgeAdjacency::new();
    let mut pending: VecDeque<((u8, u8), (u8, u8))> = VecDeque::new();

    for (p, children) in SUPERTILE_CHILDREN.iter().enumerate() {
        for ci in children.iter() {
            for (d, &dir) in DIRECTIONS.iter().enumerate() {
                if let Some(j) = child_at(p, ci.hex + dir) {
                    let cj = &SUPERTILE_CHILDREN[p][j as usize];
                    let x = (ci.type_idx, ((d + 6 - ci.rotation as usize) % 6) as u8);
                    let y = (cj.type_idx, ((d + 9 - cj.rotation as usize) % 6) as u8);
                    pending.push_back((x, y));
                }
            }
        }
    }

    while let Some((x, y)) = pending.pop_front() {
        if !rel.entry(x).or_default().insert(y) {
            continue;
        }
        pending.push_back((y, x));

        let sa = &bt.edge_seq[x.0 as usize][x.1 as usize];
        let sb = &bt.edge_seq[y.0 as usize][y.1 as usize];
        assert_eq!(
            sa.len(),
            sb.len(),
            "glued super-edges {x:?} and {y:?} have different segment counts",
        );
        for k in 0..sa.len() {
            let a = sa[k];
            let b = sb[sa.len() - 1 - k];
            let ca = &SUPERTILE_CHILDREN[x.0 as usize][a.child as usize];
            let cb = &SUPERTILE_CHILDREN[y.0 as usize][b.child as usize];
            pending.push_back((
                (ca.type_idx, ((a.dir + 6 - ca.rotation) % 6)),
                (cb.type_idx, ((b.dir + 6 - cb.rotation) % 6)),
            ));
        }
    }

    rel
}

/// Neighbor of the tile at `coords` (under top-level supertile `top`) across
/// the leaf tile's edge `edge` (0–5, in the leaf's *base* orientation).
///
/// Returns the neighbor's coordinates and the neighbor's edge (again in its
/// own base orientation) that faces back, or `None` when the move crosses
/// the top-level supertile's boundary (no answer within this context).
///
/// This is the recursive reference algorithm from Tatham's combinatorial-
/// coordinates article; `crate::transducer` implements the equivalent
/// finite-state transducer.
pub fn neighbor(top: u8, coords: &TreeCoords, edge: u8) -> Option<(TreeCoords, u8)> {
    let types = types_along(top, &coords.path);
    let (path, _, back) =
        neighbor_rec(&types, &coords.path, edge, boundary_tables())?;
    Some((TreeCoords { path }, back))
}

/// Core recursion: neighbor of the depth-`path.len()` supertile (type
/// `types[path.len()]`) across its edge `delta` (own base frame).  Returns
/// the neighbor's path, type, and back edge in the neighbor's base frame.
fn neighbor_rec(
    types: &[u8],
    path: &[u8],
    delta: u8,
    bt: &BoundaryTables,
) -> Option<(Vec<u8>, u8, u8)> {
    let m = path.len();
    if m == 0 {
        return None; // ran off the top of the hierarchy
    }
    let p = types[m - 1] as usize;
    let i = path[m - 1];
    let ch = &SUPERTILE_CHILDREN[p][i as usize];
    let d = (delta as usize + ch.rotation as usize) % 6;

    if let Some(j) = child_at(p, ch.hex + DIRECTIONS[d]) {
        // Sibling move: resolved at this level, prefix unchanged.
        let cj = &SUPERTILE_CHILDREN[p][j as usize];
        let mut out = path[..m - 1].to_vec();
        out.push(j);
        let back = ((d + 9 - cj.rotation as usize) % 6) as u8;
        return Some((out, cj.type_idx, back));
    }

    // Boundary: find which super-edge of the parent we cross and where,
    // resolve the parent's neighbor one level up, then descend into it.
    let &(e, k) = bt.seg_lookup[p].get(&(i, d as u8)).unwrap();
    let (mut out, n_type, e_back) = neighbor_rec(&types[..m], &path[..m - 1], e, bt)?;
    let seq_p = &bt.edge_seq[p][e as usize];
    let seq_n = &bt.edge_seq[n_type as usize][e_back as usize];
    assert_eq!(
        seq_p.len(),
        seq_n.len(),
        "glued super-edges ({p}, {e}) and ({n_type}, {e_back}) differ in length",
    );
    let seg = seq_n[seq_n.len() - 1 - k as usize];
    let cj = &SUPERTILE_CHILDREN[n_type as usize][seg.child as usize];
    out.push(seg.child);
    // `seg.dir` points out of the neighbor patch, i.e. straight back at us.
    let back = (seg.dir + 6 - cj.rotation) % 6;
    Some((out, cj.type_idx, back))
}

/// Tiling patch of the canonical depth-`depth` expansion of a single `top`
/// tile (via [`canonical_supersubstitute_with_placements`]), together with
/// the [`TreeCoords`] of every cell.  The ground truth that path-based
/// neighbor computations are tested against.
pub fn canonical_patch_paths(
    top: u8,
    depth: usize,
) -> (MarkedTiling<Label>, HashMap<Hex, TreeCoords>) {
    let mut tiling = MarkedTiling::new();
    tiling.insert(Hex::new(0, 0), BASE_TILES[top as usize].clone());
    let mut paths: HashMap<Hex, Vec<u8>> =
        HashMap::from([(Hex::new(0, 0), Vec::new())]);

    for _ in 0..depth {
        let (next, placements) = canonical_supersubstitute_with_placements(&tiling);
        let mut next_paths = HashMap::with_capacity(placements.len() * 8);
        for (hex, path) in &paths {
            let pl = placements[hex];
            for (idx, ch) in SUPERTILE_CHILDREN[pl.type_idx].iter().enumerate() {
                let mut cell = ch.hex;
                for _ in 0..pl.rotation {
                    cell = cell.rotate_cw();
                }
                let mut p = path.clone();
                p.push(idx as u8);
                next_paths.insert(cell + pl.offset, p);
            }
        }
        assert_eq!(next_paths.len(), next.tiles.len());
        tiling = next;
        paths = next_paths;
    }

    let paths = paths
        .into_iter()
        .map(|(h, p)| (h, TreeCoords { path: p }))
        .collect();
    (tiling, paths)
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

    /// Each patch boundary decomposes into 6 non-empty super-edges covering
    /// every boundary hex-edge exactly once (20 for Γ's 7 cells, 22 for the
    /// 8-cell patches), and the inverse lookup is consistent.
    #[test]
    fn boundary_tables_cover_boundary() {
        let bt = boundary_tables();
        for (p, children) in SUPERTILE_CHILDREN.iter().enumerate() {
            let mut expected = 0;
            for (i, c) in children.iter().enumerate() {
                for (d, &dir) in DIRECTIONS.iter().enumerate() {
                    if child_at(p, c.hex + dir).is_none() {
                        expected += 1;
                        let &(e, k) = bt.seg_lookup[p]
                            .get(&(i as u8, d as u8))
                            .unwrap_or_else(|| panic!(
                                "type {p}: boundary edge ({i}, {d}) missing from lookup"
                            ));
                        let seg = bt.edge_seq[p][e as usize][k as usize];
                        assert_eq!((seg.child, seg.dir), (i as u8, d as u8));
                    }
                }
            }
            assert_eq!(expected, if p == 0 { 20 } else { 22 }, "type {p}");
            let total: usize = bt.edge_seq[p].iter().map(|s| s.len()).sum();
            assert_eq!(total, expected, "type {p}: super-edges don't cover boundary");
        }
    }

    /// The edge-adjacency closure terminates with every pair of glued
    /// super-edges having equal segment counts (asserted inside the builder —
    /// Tatham's completeness check), and the relation is symmetric.
    #[test]
    fn edge_adjacency_is_symmetric_and_consistent() {
        let rel = edge_adjacency();
        assert!(!rel.is_empty());
        for (x, partners) in rel {
            for y in partners {
                assert!(
                    rel[y].contains(x),
                    "edge adjacency not symmetric: {x:?} ~ {y:?}",
                );
            }
        }
    }

    /// `neighbor` must agree with the actual hex-grid adjacency of the
    /// canonical geometric expansion: for every tile and every direction,
    /// either both report a neighbor with matching coordinates and back edge,
    /// or both report none (the move leaves the top-level patch).
    fn check_neighbors_against_patch(top: u8, depth: usize) {
        use crate::tiling::tile_id;

        let (tiling, paths) = canonical_patch_paths(top, depth);
        assert_eq!(tiling.tiles.len(), paths.len());

        for (&hex, coords) in &paths {
            let (ti, rho) = tile_id(&tiling.tiles[&hex]).unwrap();
            assert_eq!(
                *types_along(top, &coords.path).last().unwrap() as usize,
                ti,
                "top {top} depth {depth}: type mismatch at {hex:?}",
            );
            assert_eq!(
                path_rotation(top, &coords.path) as usize,
                rho,
                "top {top} depth {depth}: rotation mismatch at {hex:?}",
            );

            for (w, &dir) in DIRECTIONS.iter().enumerate() {
                let delta = ((w + 6 - rho) % 6) as u8;
                let nb_hex = hex + dir;
                let got = neighbor(top, coords, delta);
                match (paths.get(&nb_hex), got) {
                    (None, None) => {}
                    (Some(np), Some((gp, back))) => {
                        assert_eq!(
                            &gp, np,
                            "top {top}: wrong neighbor path from {hex:?} dir {w}",
                        );
                        let (_, nrho) = tile_id(&tiling.tiles[&nb_hex]).unwrap();
                        assert_eq!(
                            back as usize,
                            (w + 9 - nrho) % 6,
                            "top {top}: wrong back edge from {hex:?} dir {w}",
                        );
                    }
                    (expected, got) => panic!(
                        "top {top} depth {depth}: from {hex:?} dir {w}: \
                         geometry says {expected:?}, neighbor() says {got:?}",
                    ),
                }
            }
        }
    }

    #[test]
    fn neighbor_matches_geometry_depth_2_all_tops() {
        for top in 0..9 {
            check_neighbors_against_patch(top, 2);
        }
    }

    #[test]
    fn neighbor_matches_geometry_depth_3_all_tops() {
        for top in 0..9 {
            check_neighbors_against_patch(top, 3);
        }
    }

    #[test]
    fn neighbor_matches_geometry_depth_4_gamma() {
        check_neighbors_against_patch(0, 4);
    }

    /// ~27k tiles, ~161k queries; the deepest direct confirmation that the
    /// level-1 boundary tables stay valid at higher substitution levels.
    #[test]
    fn neighbor_matches_geometry_depth_5_gamma() {
        check_neighbors_against_patch(0, 5);
    }

    /// The all-zeros path is the nested-GAMMA chain pinned at the origin
    /// cell.  Crossing its W/NW edges hits a supertile boundary at every
    /// level — an "infinite wall" in Tatham's terms — so within any finite
    /// context the answer is `None`, while the other four edges resolve
    /// immediately to siblings.
    #[test]
    fn gamma_chain_has_infinite_wall() {
        for depth in [1, 5, 40] {
            let coords = TreeCoords { path: vec![0; depth] };
            for delta in [2, 3] {
                assert_eq!(neighbor(0, &coords, delta), None, "depth {depth}");
            }
            for delta in [0, 1, 4, 5] {
                let (got, _) = neighbor(0, &coords, delta).unwrap();
                assert_eq!(got.path[..depth - 1], coords.path[..depth - 1]);
                assert_ne!(got.path[depth - 1], 0);
            }
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
