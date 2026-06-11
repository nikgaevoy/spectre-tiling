//! Finite-state transducer for [`TreeCoords`] neighbor computation.
//!
//! Replaces the recursive transition algorithm ([`crate::tree_coords::neighbor`])
//! with a deterministic transducer in the style of Tatham's "Beyond the Wall:
//! Working with Aperiodic Tilings Using Finite-State Transducers" (see
//! `references/sgtatham_transducers_article.md`): the machine reads the
//! coordinate path one `(parent type, child index)` symbol at a time,
//! leaf-to-root, and emits the neighbor's path with a bounded lag.
//!
//! While the move keeps crossing supertile boundaries (the "carry"), the
//! output at the levels already read is not yet determined — it depends on
//! which supertile turns out to be on the other side.  A deterministic state
//! therefore holds, for every possible resolution `(neighbor type, its back
//! edge)`, the output symbols still pending for that branch.  After each
//! input symbol the common prefix shared by all branches is emitted
//! immediately; only the differing suffixes stay pending.  The construction
//! terminates because the required lookahead is bounded for this substitution
//! system, which the builder verifies by exhausting the reachable state space
//! under a safety cap.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::OnceLock;

use crate::hex::DIRECTIONS;
use crate::tree_coords::{
    SUPERTILE_CHILDREN, TreeCoords, boundary_tables, child_at, edge_adjacency, types_along,
};

/// Input symbol: `(parent supertile type, child index within it)`.
/// A path is fed leaf-to-root, so the symbol for the leaf comes first.
pub type Sym = (u8, u8);

/// Pending data for one carry-resolution branch: the output symbols
/// (leaf-first) not yet emitted on this branch, and the back edge of the
/// final neighbor tile (in its own base orientation).
type Branch = (Vec<u8>, u8);

/// One [`Branch`] per possible carry resolution `(neighbor type, back edge)`.
type Residues = BTreeMap<(u8, u8), Branch>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum State {
    /// Nothing read yet; the query is "cross edge δ of the leaf tile",
    /// δ in the leaf's base orientation.
    Start(u8),
    /// The carry has resolved; every remaining input symbol is echoed.
    Copy,
    /// The move still crosses supertile boundaries.  We owe the levels above
    /// an answer to: "what lies across super-edge `e` (own base frame) of the
    /// deepest unresolved supertile, of type `t`?"  `residues` is keyed by
    /// the possible answers, exactly the pairs `edge_adjacency()[(t, e)]`.
    Carry { t: u8, e: u8, residues: Residues },
}

/// One deterministic transition: successor state, output symbols emitted
/// (leaf-first), and the final back edge if the carry resolved here.
#[derive(Debug, Clone)]
struct Step {
    next: usize,
    emit: Vec<u8>,
    back: Option<u8>,
}

/// The compiled transducer: a transition table over interned states.
pub struct Transducer {
    steps: Vec<HashMap<Sym, Step>>,
    start: [usize; 6],
    copy: usize,
}

/// Emit the longest output prefix shared by every branch.
fn strip_common_prefix(residues: &mut Residues) -> Vec<u8> {
    let mut emit = Vec::new();
    while let Some(&sym) = residues.values().next().and_then(|(s, _)| s.first()) {
        if !residues.values().all(|(s, _)| s.first() == Some(&sym)) {
            break;
        }
        for (s, _) in residues.values_mut() {
            s.remove(0);
        }
        emit.push(sym);
    }
    emit
}

/// The deterministic transition function.  Returns `None` only for input
/// symbols inconsistent with the state (the child's type does not match the
/// supertile type whose neighbor is being resolved) — such symbols cannot
/// occur in a well-formed coordinate path.
fn transition(state: &State, sym: Sym) -> Option<(State, Vec<u8>, Option<u8>)> {
    let bt = boundary_tables();
    let rel = edge_adjacency();
    let (p, i) = sym;
    let ch = &SUPERTILE_CHILDREN[p as usize][i as usize];

    // Shared boundary-crossing step: the object we just read is child `i` of
    // a `p`-typed supertile and crosses that supertile's super-edge `e2` at
    // segment `k`; build one pending branch per possible far side of `e2`.
    let carry_on = |e2: u8, k: u8, lookup: &dyn Fn(&(u8, u8)) -> Branch| {
        let mut residues = Residues::new();
        for &(n2, e2b) in &rel[&(p, e2)] {
            let seq = &bt.edge_seq[n2 as usize][e2b as usize];
            let seg = seq[seq.len() - 1 - k as usize];
            let cj = &SUPERTILE_CHILDREN[n2 as usize][seg.child as usize];
            // `seg.dir` points out of the far patch, straight back at us.
            let key = (cj.type_idx, (seg.dir + 6 - cj.rotation) % 6);
            let (mut s, back) = lookup(&key);
            s.push(seg.child);
            residues.insert((n2, e2b), (s, back));
        }
        assert!(
            !residues.is_empty(),
            "no candidate far side for ({p}, {e2})"
        );
        let mut st = State::Carry {
            t: p,
            e: e2,
            residues,
        };
        let emit = match &mut st {
            State::Carry { residues, .. } => strip_common_prefix(residues),
            _ => unreachable!(),
        };
        (st, emit, None)
    };

    match state {
        State::Copy => Some((State::Copy, vec![i], None)),

        State::Start(delta) => {
            let d = (*delta as usize + ch.rotation as usize) % 6;
            match child_at(p as usize, ch.hex + DIRECTIONS[d]) {
                Some(j) => {
                    // Sibling move: resolved immediately.
                    let cj = &SUPERTILE_CHILDREN[p as usize][j as usize];
                    let back = ((d + 9 - cj.rotation as usize) % 6) as u8;
                    Some((State::Copy, vec![j], Some(back)))
                }
                None => {
                    let &(e2, k) = bt.seg_lookup[p as usize].get(&(i, d as u8)).unwrap();
                    // The leaf is the only level below: each branch starts
                    // with just the new leaf index and its back edge.
                    Some(carry_on(e2, k, &|key: &(u8, u8)| (Vec::new(), key.1)))
                }
            }
        }

        State::Carry { t, e, residues } => {
            if ch.type_idx != *t {
                return None;
            }
            let d = (*e as usize + ch.rotation as usize) % 6;
            match child_at(p as usize, ch.hex + DIRECTIONS[d]) {
                Some(j) => {
                    // Sibling move one level up: the carry resolves, and the
                    // sibling's (type, back edge) selects the true branch.
                    let cj = &SUPERTILE_CHILDREN[p as usize][j as usize];
                    let key = (cj.type_idx, ((d + 9 - cj.rotation as usize) % 6) as u8);
                    let (s, back) = residues
                        .get(&key)
                        .unwrap_or_else(|| panic!("missing residue branch {key:?}"))
                        .clone();
                    let mut emit = s;
                    emit.push(j);
                    Some((State::Copy, emit, Some(back)))
                }
                None => {
                    let &(e2, k) = bt.seg_lookup[p as usize].get(&(i, d as u8)).unwrap();
                    Some(carry_on(e2, k, &|key: &(u8, u8)| {
                        residues
                            .get(key)
                            .unwrap_or_else(|| panic!("missing residue branch {key:?}"))
                            .clone()
                    }))
                }
            }
        }
    }
}

impl Transducer {
    /// The transducer for this substitution system (built once, on first use).
    pub fn global() -> &'static Transducer {
        static T: OnceLock<Transducer> = OnceLock::new();
        T.get_or_init(Transducer::build)
    }

    /// Determinise the transition relation by exploring every state reachable
    /// from the six start states under every consistent input symbol.  Doubles
    /// as the completeness check: any gap in the tables (a reachable demand
    /// with no candidate far side, or a missing residue branch) panics here.
    pub fn build() -> Transducer {
        const STATE_CAP: usize = 10_000;

        fn intern(
            st: State,
            ids: &mut HashMap<State, usize>,
            queue: &mut VecDeque<(usize, State)>,
        ) -> usize {
            if let Some(&id) = ids.get(&st) {
                return id;
            }
            let id = ids.len();
            assert!(id < STATE_CAP, "transducer construction does not terminate");
            ids.insert(st.clone(), id);
            queue.push_back((id, st));
            id
        }

        let mut ids: HashMap<State, usize> = HashMap::new();
        let mut queue: VecDeque<(usize, State)> = VecDeque::new();
        let copy = intern(State::Copy, &mut ids, &mut queue);
        let start = std::array::from_fn(|d| intern(State::Start(d as u8), &mut ids, &mut queue));

        let mut steps: HashMap<usize, HashMap<Sym, Step>> = HashMap::new();
        while let Some((id, st)) = queue.pop_front() {
            let mut row = HashMap::new();
            for p in 0..9u8 {
                for i in 0..SUPERTILE_CHILDREN[p as usize].len() as u8 {
                    if let Some((next, emit, back)) = transition(&st, (p, i)) {
                        let next = intern(next, &mut ids, &mut queue);
                        row.insert((p, i), Step { next, emit, back });
                    }
                }
            }
            steps.insert(id, row);
        }

        let mut table = vec![HashMap::new(); ids.len()];
        for (id, row) in steps {
            table[id] = row;
        }
        Transducer {
            steps: table,
            start,
            copy,
        }
    }

    /// Number of deterministic states (including `Copy` and the 6 starts).
    pub fn state_count(&self) -> usize {
        self.steps.len()
    }

    /// Neighbor of the tile at `coords` (under top-level supertile `top`)
    /// across the leaf tile's edge `edge` (0–5, in the leaf's base
    /// orientation).  Equivalent to [`crate::tree_coords::neighbor`], but
    /// runs in one pass over the path with O(1) work per level.
    ///
    /// Returns the neighbor's coordinates and its back edge, or `None` when
    /// the move crosses the top-level supertile's boundary.
    pub fn neighbor(&self, top: u8, coords: &TreeCoords, edge: u8) -> Option<(TreeCoords, u8)> {
        let types = types_along(top, &coords.path);
        let mut state = self.start[edge as usize];
        let mut out: Vec<u8> = Vec::with_capacity(coords.path.len());
        let mut back = None;
        for l in (0..coords.path.len()).rev() {
            let step = self.steps[state].get(&(types[l], coords.path[l]))?;
            out.extend_from_slice(&step.emit);
            back = back.or(step.back);
            state = step.next;
        }
        if state != self.copy {
            return None; // carry never resolved within this context
        }
        debug_assert_eq!(out.len(), coords.path.len());
        out.reverse();
        Some((
            TreeCoords { path: out },
            back.expect("resolved without back edge"),
        ))
    }

    /// Order of the supertile boundary crossed when leaving the tile at
    /// `coords` through edge `edge`: the number of levels the carry stays
    /// unresolved.  0 means a plain tile border (the neighbor is a sibling
    /// within the same level-1 supertile); `k` means the edge lies on the
    /// boundary of an order-`k` supertile but not an order-`k+1` one;
    /// `coords.depth()` means the move runs off the top of the context
    /// (the patch's outer rim).
    pub fn border_order(&self, top: u8, coords: &TreeCoords, edge: u8) -> usize {
        let types = types_along(top, &coords.path);
        let mut state = self.start[edge as usize];
        for (consumed, l) in (0..coords.path.len()).rev().enumerate() {
            let Some(step) = self.steps[state].get(&(types[l], coords.path[l])) else {
                break; // inconsistent path: treat as never resolving
            };
            if step.back.is_some() {
                return consumed;
            }
            state = step.next;
        }
        coords.path.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree_coords::{canonical_patch_paths, neighbor};

    /// The transducer must agree with the recursive reference algorithm on
    /// every tile of every canonical patch, in all 6 directions — including
    /// the `None` cases at the patch boundary.  (The reference algorithm is
    /// itself validated against raw geometry in `tree_coords::tests`.)
    fn check_against_recursive(top: u8, depth: usize) {
        let t = Transducer::global();
        let (_, paths) = canonical_patch_paths(top, depth);
        for coords in paths.values() {
            for edge in 0..6 {
                assert_eq!(
                    t.neighbor(top, coords, edge),
                    neighbor(top, coords, edge),
                    "top {top} depth {depth}: mismatch at {:?} edge {edge}",
                    coords.path,
                );
            }
        }
    }

    #[test]
    fn matches_recursive_depth_3_all_tops() {
        for top in 0..9 {
            check_against_recursive(top, 3);
        }
    }

    #[test]
    fn matches_recursive_depth_4_gamma() {
        check_against_recursive(0, 4);
    }

    /// Construction terminates (bounded lookahead) with a stable number of
    /// deterministic states.  The exact count is empirical; a change means
    /// the substitution tables changed.
    #[test]
    fn state_count_is_stable() {
        let n = Transducer::global().state_count();
        assert_eq!(n, 279, "transducer state count drifted");
    }

    /// `border_order` must equal the divergence point of the two paths: with
    /// a neighbor, depth minus the common prefix length minus 1; without one
    /// (the move leaves the patch), the full depth.
    #[test]
    fn border_order_matches_path_divergence() {
        let t = Transducer::global();
        for top in 0..9 {
            let (_, paths) = canonical_patch_paths(top, 3);
            for coords in paths.values() {
                for edge in 0..6 {
                    let order = t.border_order(top, coords, edge);
                    match t.neighbor(top, coords, edge) {
                        Some((nb, _)) => {
                            let common = coords
                                .path
                                .iter()
                                .zip(&nb.path)
                                .take_while(|(a, b)| a == b)
                                .count();
                            assert_eq!(
                                order,
                                coords.path.len() - common - 1,
                                "top {top}: {:?} edge {edge}",
                                coords.path,
                            );
                        }
                        None => assert_eq!(
                            order,
                            coords.path.len(),
                            "top {top}: {:?} edge {edge}",
                            coords.path,
                        ),
                    }
                }
            }
        }
    }

    /// Deterministic LCG so the test reproduces without dependencies.
    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state >> 33
    }

    /// Random paths up to depth 40 — far beyond what the geometric tests can
    /// reach — checked for three independent invariants: the transducer
    /// agrees with the recursive algorithm (including `None`s), the two
    /// facing edges carry negated labels (the marked-tile gluing rule, which
    /// does not derive from the boundary tables), and crossing back from the
    /// neighbor returns home across the same edge.
    #[test]
    fn deep_random_paths_agree_glue_and_reciprocate() {
        use crate::tiling::BASE_TILES;

        let t = Transducer::global();
        let mut rng: u64 = 0x5EED_0BAD_CAFE;
        for case in 0..2000 {
            let top = (lcg(&mut rng) % 9) as u8;
            let depth = 1 + (lcg(&mut rng) % 40) as usize;
            let mut path = Vec::with_capacity(depth);
            let mut ty = top as usize;
            for _ in 0..depth {
                let n = SUPERTILE_CHILDREN[ty].len() as u64;
                let c = (lcg(&mut rng) % n) as u8;
                path.push(c);
                ty = SUPERTILE_CHILDREN[ty][c as usize].type_idx as usize;
            }
            let coords = TreeCoords { path };
            let leaf_type = ty;

            for edge in 0..6u8 {
                let rec = neighbor(top, &coords, edge);
                let fst = t.neighbor(top, &coords, edge);
                assert_eq!(rec, fst, "case {case}: transducer != recursive");
                let Some((nb, back)) = rec else { continue };
                assert_eq!(nb.path.len(), coords.path.len(), "case {case}");

                let nb_type = *types_along(top, &nb.path).last().unwrap() as usize;
                assert_eq!(
                    BASE_TILES[leaf_type].edges[edge as usize],
                    -BASE_TILES[nb_type].edges[back as usize],
                    "case {case} edge {edge}: incompatible edge labels",
                );

                assert_eq!(
                    t.neighbor(top, &nb, back),
                    Some((coords.clone(), edge)),
                    "case {case} edge {edge}: not reciprocal",
                );
            }
        }
    }

    /// On the nested-GAMMA "infinite wall" input (the all-zeros path crossed
    /// on its W edge) the carry never resolves and the machine must cycle
    /// through a bounded set of carry states — the property that makes the
    /// infinitary (eventually-periodic) transition algorithm possible.
    #[test]
    fn wall_input_cycles_states() {
        let t = Transducer::global();
        let coords = TreeCoords { path: vec![0; 100] };
        assert_eq!(t.neighbor(0, &coords, 2), None);

        // Trace the state sequence by hand and find a repeat.
        let mut state = t.start[2];
        let mut seen = vec![state];
        let mut cycle = None;
        for _ in 0..100 {
            let step = &t.steps[state][&(0, 0)]; // (Γ, child 0) forever
            state = step.next;
            if let Some(pos) = seen.iter().position(|&s| s == state) {
                cycle = Some(seen.len() - pos);
                break;
            }
            seen.push(state);
        }
        let period = cycle.expect("no state cycle found on periodic wall input");
        println!("wall input enters a cycle of period {period}");
    }
}
