use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

use eframe::egui;
use spectre_tiling::hex::{DIRECTIONS, Hex};
use spectre_tiling::marked::{MarkedTile, MarkedTiling};
use spectre_tiling::spectre::Label;
use spectre_tiling::spectre_geom::{
    SPECMAP, SPECTRE_TRIANGLES, Zd, our_direction, spectre_place, spectre_step,
};
use spectre_tiling::supertile::{
    supertile_delta, supertile_gamma, supertile_lambda, supertile_phi, supertile_pi, supertile_psi,
    supertile_sigma, supertile_theta, supertile_xi,
};
use spectre_tiling::tiling::{
    BASE_TILES, TILE_NAMES, placement_cells, supersubstitute_with_placements, tile_id,
};
use spectre_tiling::transducer::Transducer;
use spectre_tiling::tree_coords::{SUPERTILE_CHILDREN, TreeCoords, types_along};

const TILE_COLORS: [egui::Color32; 9] = [
    egui::Color32::from_rgb(0xc8, 0x78, 0x28), // Γ
    egui::Color32::from_rgb(0x8b, 0x3a, 0x10), // Δ
    egui::Color32::from_rgb(0x4a, 0x90, 0x90), // Θ
    egui::Color32::from_rgb(0x3a, 0x80, 0x50), // Λ
    egui::Color32::from_rgb(0xc8, 0xa8, 0x48), // Ξ
    egui::Color32::from_rgb(0xc0, 0x40, 0x60), // Π
    egui::Color32::from_rgb(0x48, 0x78, 0xa0), // Σ
    egui::Color32::from_rgb(0xa8, 0xc8, 0x28), // Φ
    egui::Color32::from_rgb(0x78, 0x98, 0x48), // Ψ
];

// Border palettes by order, 1-based; deeper orders clamp to the last entry.
// All are sequential ramps so adjacent orders read as a hierarchy.
const PALETTE_HEAT: [egui::Color32; 7] = [
    egui::Color32::WHITE,
    egui::Color32::from_rgb(0xff, 0xcc, 0x33), // gold
    egui::Color32::from_rgb(0xff, 0x8c, 0x1a), // orange
    egui::Color32::from_rgb(0xf2, 0x3d, 0x3d), // red
    egui::Color32::from_rgb(0xcc, 0x1f, 0x4d), // crimson
    egui::Color32::from_rgb(0xb3, 0x2d, 0xb5), // magenta
    egui::Color32::from_rgb(0x77, 0x33, 0xcc), // violet
];
const PALETTE_ICE: [egui::Color32; 7] = [
    egui::Color32::WHITE,
    egui::Color32::from_rgb(0xa8, 0xe6, 0xf0), // pale cyan
    egui::Color32::from_rgb(0x5a, 0xb4, 0xf0), // sky
    egui::Color32::from_rgb(0x2d, 0x7d, 0xf2), // azure
    egui::Color32::from_rgb(0x4d, 0x4d, 0xd9), // indigo
    egui::Color32::from_rgb(0x80, 0x33, 0xcc), // violet
    egui::Color32::from_rgb(0xb3, 0x33, 0xa6), // plum
];
const PALETTE_VIRIDIS: [egui::Color32; 7] = [
    egui::Color32::from_rgb(0xfd, 0xe7, 0x25), // yellow
    egui::Color32::from_rgb(0x90, 0xd7, 0x43), // lime
    egui::Color32::from_rgb(0x35, 0xb7, 0x79), // green
    egui::Color32::from_rgb(0x21, 0x91, 0x8c), // teal
    egui::Color32::from_rgb(0x31, 0x68, 0x8e), // steel blue
    egui::Color32::from_rgb(0x44, 0x39, 0x83), // indigo
    egui::Color32::from_rgb(0x44, 0x01, 0x54), // deep purple
];
const PALETTE_GOLD: [egui::Color32; 7] = [
    egui::Color32::WHITE,
    egui::Color32::from_rgb(0xff, 0xe9, 0xb3), // cream
    egui::Color32::from_rgb(0xff, 0xd2, 0x4d), // gold
    egui::Color32::from_rgb(0xe6, 0xa3, 0x23), // amber
    egui::Color32::from_rgb(0xb3, 0x77, 0x00), // bronze
    egui::Color32::from_rgb(0x80, 0x55, 0x00), // umber
    egui::Color32::from_rgb(0x4d, 0x33, 0x00), // dark umber
];

// Color scheme for the order-coded supertile borders.
#[derive(Debug, PartialEq, Clone, Copy)]
enum BorderPalette {
    // No coding: uniform white over colored tiles, dark over plain ones.
    Plain,
    Heat,
    Ice,
    Viridis,
    Gold,
}

impl BorderPalette {
    const ALL: [BorderPalette; 5] = [
        BorderPalette::Plain,
        BorderPalette::Heat,
        BorderPalette::Ice,
        BorderPalette::Viridis,
        BorderPalette::Gold,
    ];

    fn name(self) -> &'static str {
        match self {
            BorderPalette::Plain => "Plain",
            BorderPalette::Heat => "Heat",
            BorderPalette::Ice => "Ice",
            BorderPalette::Viridis => "Viridis",
            BorderPalette::Gold => "Gold fade",
        }
    }

    fn colors(self) -> Option<&'static [egui::Color32; 7]> {
        match self {
            BorderPalette::Plain => None,
            BorderPalette::Heat => Some(&PALETTE_HEAT),
            BorderPalette::Ice => Some(&PALETTE_ICE),
            BorderPalette::Viridis => Some(&PALETTE_VIRIDIS),
            BorderPalette::Gold => Some(&PALETTE_GOLD),
        }
    }
}

// The three published shapes of the tile (Fig. 1.1 of the spectre paper):
// the straight-edged 14-gon Tile(1,1), and two Spectres obtained from it by
// replacing every edge with a curve.  Any curve symmetric under 180°
// rotation about the edge midpoint traces the same arc from both sides of a
// glued edge, so the tiling itself is untouched — only the outline drawn
// for each tile changes.
#[derive(Debug, PartialEq, Clone, Copy)]
enum EdgeStyle {
    // Straight unit edges: the polygon Tile(1,1).
    Tile11,
    // Each edge a double wave (the paper's Fig. 1.1, centre).
    Wiggly,
    // Each edge one smooth S-curve (the paper's Fig. 1.1, right).
    Smooth,
}

impl EdgeStyle {
    const ALL: [EdgeStyle; 3] = [EdgeStyle::Tile11, EdgeStyle::Wiggly, EdgeStyle::Smooth];

    fn name(self) -> &'static str {
        match self {
            EdgeStyle::Tile11 => "Tile(1,1)",
            EdgeStyle::Wiggly => "Spectre (wiggly)",
            EdgeStyle::Smooth => "Spectre (smooth)",
        }
    }

    fn index(self) -> usize {
        match self {
            EdgeStyle::Tile11 => 0,
            EdgeStyle::Wiggly => 1,
            EdgeStyle::Smooth => 2,
        }
    }

    // Outline points contributed per edge (each edge owns its starting
    // corner, so straight edges need just the corner itself).
    fn samples(self) -> usize {
        match self {
            EdgeStyle::Tile11 => 1,
            EdgeStyle::Wiggly | EdgeStyle::Smooth => 12,
        }
    }

    // Perpendicular deviation of the edge curve at parameter t ∈ [0, 1], in
    // units of the edge length.  Odd around the midpoint (f(1−t) = −f(t)),
    // which is exactly the gluing condition above.
    fn offset(self, t: f32) -> f32 {
        use std::f32::consts::TAU;
        match self {
            EdgeStyle::Tile11 => 0.0,
            EdgeStyle::Wiggly => 0.13 * (2.0 * TAU * t).sin(),
            EdgeStyle::Smooth => 0.22 * (TAU * t).sin(),
        }
    }
}

// Supertile constructors in the same order as TILE_NAMES / BASE_TILES.
const BASE_SUPERTILE_FNS: [fn() -> MarkedTiling<Label>; 9] = [
    supertile_gamma,
    supertile_delta,
    supertile_theta,
    supertile_lambda,
    supertile_xi,
    supertile_pi,
    supertile_sigma,
    supertile_phi,
    supertile_psi,
];

// Rotate an entire tiling by `n` CCW visual steps (60° each).
// hex.rotate_cw() moves a hex CCW on screen (y-down coords); tile.rotate(n) does the same
// for edge labels — both transformations are consistent so validity is preserved.
fn rotate_tiling(tiling: &MarkedTiling<Label>, n: usize) -> MarkedTiling<Label> {
    let mut result = MarkedTiling::new();
    for (&hex, tile) in &tiling.tiles {
        let mut rh = hex;
        for _ in 0..n {
            rh = rh.rotate_cw();
        }
        result.insert(rh, tile.rotate(n));
    }
    result
}

// Pointy-top hex: corner i is at angle (30 + 60*i) degrees.
fn corner(sc: egui::Pos2, zoom: f32, i: usize) -> egui::Pos2 {
    let a = std::f32::consts::PI / 180.0 * (30.0 + 60.0 * i as f32);
    sc + egui::vec2(zoom * a.cos(), zoom * a.sin())
}

fn hex_corners(sc: egui::Pos2, zoom: f32) -> Vec<egui::Pos2> {
    (0..6).map(|i| corner(sc, zoom, i)).collect()
}

// Edge i (facing DIRECTIONS[i]) spans corners (11-i)%6 and (6-i)%6.
// Verified: E→corners 5,0; NE→4,5; NW→3,4; W→2,3; SW→1,2; SE→0,1.
fn edge_endpoints(sc: egui::Pos2, zoom: f32, i: usize) -> [egui::Pos2; 2] {
    [
        corner(sc, zoom, (11 - i) % 6),
        corner(sc, zoom, (6 - i) % 6),
    ]
}

fn hex_to_screen(hex: Hex, zoom: f32, pan: egui::Vec2, canvas_center: egui::Pos2) -> egui::Pos2 {
    let sqrt3 = 3f32.sqrt();
    let wx = sqrt3 * hex.q as f32 + sqrt3 / 2.0 * hex.r as f32;
    let wy = -1.5 * hex.r as f32;
    canvas_center + pan + egui::vec2(wx * zoom, wy * zoom)
}

// Spectre-plane point (math y-up, unit = spectre edge) to screen.
fn zd_screen(p: Zd, zoom: f32, pan: egui::Vec2, canvas_center: egui::Pos2) -> egui::Pos2 {
    let (x, y) = p.to_xy();
    canvas_center + pan + egui::vec2(x as f32 * zoom, -y as f32 * zoom)
}

// Ray-casting point-in-polygon test (the spectre is concave, so a convex
// test won't do).
fn point_in_polygon(p: egui::Pos2, pts: &[egui::Pos2]) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

// Sampled outline of a spectre with the given 14 corner positions: edge i
// contributes points [i·S, (i+1)·S), starting at corner i.  On screen every
// placed tile is a rotation/translation of one shape (the tiling never
// mirrors it), so this indexing — and hence one triangulation per style —
// fits all tiles.
fn spectre_outline(pts: &[egui::Pos2; 14], style: EdgeStyle) -> Vec<egui::Pos2> {
    let s = style.samples();
    let mut out = Vec::with_capacity(14 * s);
    for i in 0..14 {
        let (a, b) = (pts[i], pts[(i + 1) % 14]);
        let d = b - a;
        let perp = egui::vec2(-d.y, d.x);
        for k in 0..s {
            let t = k as f32 / s as f32;
            out.push(a + d * t + perp * style.offset(t));
        }
    }
    out
}

// Ear-clipping triangulation of a simple polygon, as index triples.
fn ear_clip(pts: &[egui::Pos2]) -> Vec<[u32; 3]> {
    fn cross(o: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
        (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
    }
    let n = pts.len();
    let mut idx: Vec<u32> = (0..n as u32).collect();
    let mut tris = Vec::with_capacity(n - 2);
    // The signed area fixes which turn direction counts as convex.
    let area2: f32 = (0..n)
        .map(|i| cross(egui::Pos2::ZERO, pts[i], pts[(i + 1) % n]))
        .sum();
    let sign = if area2 >= 0.0 { 1.0 } else { -1.0 };
    let mut i = 0;
    let mut stuck = 0;
    while idx.len() > 3 {
        let m = idx.len();
        let (pi, ci, ni) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
        let (p, c, q) = (pts[pi as usize], pts[ci as usize], pts[ni as usize]);
        // An ear: a convex corner whose triangle contains no other vertex.
        let ear = sign * cross(p, c, q) >= 0.0
            && idx.iter().all(|&j| {
                j == pi || j == ci || j == ni || {
                    let x = pts[j as usize];
                    !(sign * cross(p, c, x) > 0.0
                        && sign * cross(c, q, x) > 0.0
                        && sign * cross(q, p, x) > 0.0)
                }
            });
        // `stuck` breaks numerically degenerate stalemates: after a full
        // fruitless lap, clip anyway rather than loop forever.
        if ear || stuck > m {
            tris.push([pi, ci, ni]);
            idx.remove(i);
            stuck = 0;
        } else {
            i += 1;
            stuck += 1;
        }
        if i >= idx.len() {
            i = 0;
        }
    }
    tris.push([idx[0], idx[1], idx[2]]);
    tris
}

// Triangulation of the canonical outline, computed once per style — valid
// for every placed spectre, see `spectre_outline`.
fn style_triangles(style: EdgeStyle) -> &'static [[u32; 3]] {
    static CACHE: [OnceLock<Vec<[u32; 3]>>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    CACHE[style.index()].get_or_init(|| {
        if style == EdgeStyle::Tile11 {
            return SPECTRE_TRIANGLES
                .iter()
                .map(|tri| tri.map(|i| i as u32))
                .collect();
        }
        let poly = spectre_place(Zd::ZERO, Zd::ONE, 0);
        // Same y-down frame as the screen, so the winding matches too.
        let pts: [egui::Pos2; 14] = std::array::from_fn(|i| {
            let (x, y) = poly[i].to_xy();
            egui::pos2(x as f32, -y as f32)
        });
        ear_clip(&spectre_outline(&pts, style))
    })
}

fn screen_to_hex(pos: egui::Pos2, zoom: f32, pan: egui::Vec2, canvas_center: egui::Pos2) -> Hex {
    let v = pos - canvas_center - pan;
    let wx = v.x / zoom;
    let wy = v.y / zoom;
    let fr = -wy / 1.5;
    let fq = wx / 3f32.sqrt() - fr / 2.0;
    let fs = -fq - fr;
    let (mut rq, mut rr, rs) = (fq.round(), fr.round(), fs.round());
    let (dq, dr, ds) = ((rq - fq).abs(), (rr - fr).abs(), (rs - fs).abs());
    if dq > dr && dq > ds {
        rq = -rr - rs;
    } else if dr > ds {
        rr = -rq - rs;
    }
    Hex::new(rq as i32, rr as i32)
}

fn visible_hexes(rect: egui::Rect, zoom: f32, pan: egui::Vec2) -> Vec<Hex> {
    let cx = rect.center().x;
    let cy = rect.center().y;
    let to_wx = |sx: f32| (sx - cx - pan.x) / zoom;
    let to_wy = |sy: f32| (sy - cy - pan.y) / zoom;
    let wy_min = to_wy(rect.top());
    let wy_max = to_wy(rect.bottom());
    let r_min = (-wy_max / 1.5).floor() as i32 - 1;
    let r_max = (-wy_min / 1.5).ceil() as i32 + 1;
    let sqrt3 = 3f32.sqrt();
    let wx_min = to_wx(rect.left());
    let wx_max = to_wx(rect.right());
    let mut out = Vec::new();
    for r in r_min..=r_max {
        let rf = r as f32;
        let q_min = (wx_min / sqrt3 - rf / 2.0).floor() as i32 - 1;
        let q_max = (wx_max / sqrt3 - rf / 2.0).ceil() as i32 + 1;
        for q in q_min..=q_max {
            out.push(Hex::new(q, r));
        }
    }
    out
}

fn recompute_invalid(tiling: &MarkedTiling<Label>) -> HashSet<(Hex, usize)> {
    let mut bad = HashSet::new();
    for (&hex, tile) in &tiling.tiles {
        for (i, &dir) in DIRECTIONS.iter().enumerate() {
            if let Some(nbr) = tiling.tiles.get(&(hex + dir)) {
                let opp = (i + 3) % 6;
                if tile.edges[i] != -nbr.edges[opp] {
                    bad.insert((hex, i));
                    bad.insert((hex + dir, opp));
                }
            }
        }
    }
    bad
}

// TreeCoords tracking.  Present only while the tiling is a single coherent
// substitution patch: seeded when a tile or supertile is placed on an empty
// canvas, recomputed by Supersubstitute, dropped on any free-form edit.
struct TreeState {
    // Type of the top-level supertile the paths descend from.
    top: u8,
    coords: HashMap<Hex, TreeCoords>,
    // False right after placement (paths are definitional); true once the
    // paths have been recomputed by the transducer walk.
    via_transducer: bool,
}

// All TreeCoords of `tiling` recovered from a single known seed by walking
// hex adjacency with the finite-state transducer: a tile's world rotation
// (from `tile_id`) converts each world direction into a leaf edge in the
// tile's base frame, and `Transducer::neighbor` yields the adjacent path.
// Returns `None` unless the walk covers every tile consistently — i.e. the
// tiling really is one substitution patch under top-level type `top`.
fn transducer_coords(
    top: u8,
    seed_hex: Hex,
    seed: TreeCoords,
    tiling: &MarkedTiling<Label>,
) -> Option<HashMap<Hex, TreeCoords>> {
    let t = Transducer::global();
    let mut coords = HashMap::with_capacity(tiling.tiles.len());
    coords.insert(seed_hex, seed);
    let mut queue = VecDeque::from([seed_hex]);
    while let Some(hex) = queue.pop_front() {
        let (_, rho) = tile_id(tiling.tiles.get(&hex)?)?;
        let cur = coords[&hex].clone();
        for (w, &dir) in DIRECTIONS.iter().enumerate() {
            let nb = hex + dir;
            if !tiling.tiles.contains_key(&nb) {
                continue;
            }
            let delta = ((w + 6 - rho) % 6) as u8;
            let Some((nc, _)) = t.neighbor(top, &cur, delta) else {
                continue;
            };
            match coords.get(&nb) {
                Some(prev) if *prev != nc => return None,
                Some(_) => {}
                None => {
                    coords.insert(nb, nc);
                    queue.push_back(nb);
                }
            }
        }
    }
    (coords.len() == tiling.tiles.len()).then_some(coords)
}

const SUPERSCRIPTS: [char; 8] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷'];

// "Δ⁴" — one TreeCoords step: a child's tile type with its index within the
// parent supertile as a superscript (types alone are ambiguous because a
// supertile can contain two children of the same type).
fn step_label(t: u8, i: u8) -> String {
    format!("{}{}", TILE_NAMES[t as usize], SUPERSCRIPTS[i as usize])
}

// Full path: the top tile's name, then one step label per level down to
// the tile — a single letter is the unexpanded top tile itself.
fn coords_str(top: u8, c: &TreeCoords) -> String {
    let types = types_along(top, &c.path);
    let mut s = TILE_NAMES[top as usize].to_string();
    for (&i, &t) in c.path.iter().zip(&types[1..]) {
        s.push_str(&step_label(t, i));
    }
    s
}

// One generated tile in the generated modes, derived by the transducer.
#[derive(Clone)]
struct TreeTile {
    coords: TreeCoords,
    type_idx: u8,
    // World rotation; the embedding is pinned by the (0,0) tile having
    // rotation 0.
    rot: u8,
}

fn label_str(label: Label) -> &'static str {
    match label {
        Label::Alpha => "α",
        Label::NegAlpha => "-α",
        Label::Beta => "β",
        Label::NegBeta => "-β",
        Label::Gamma => "γ",
        Label::NegGamma => "-γ",
        Label::Delta => "δ",
        Label::NegDelta => "-δ",
        Label::Epsilon => "ε",
        Label::NegEpsilon => "-ε",
        Label::Zeta => "ζ",
        Label::NegZeta => "-ζ",
        Label::Theta => "θ",
        Label::NegTheta => "-θ",
        Label::Eta => "η",
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    // Generated mode showing the marked-hexagon metatiles: no stored
    // tiling — the picture is derived on the fly by the transducer from the
    // TreeCoords of the tile pinned at hex (0,0).
    MarkedHex,
    // The same generated tiling, rendered as actual spectre tiles (one per
    // hexagon, two for Γ) placed by BFS gluing in the spectre plane.
    Spectre,
    // Free-form editor over a stored hex tiling.
    HexTiling,
}

// What a click places in Hex-tiling mode.
#[derive(PartialEq, Clone, Copy)]
enum PlaceMode {
    Single,
    Supertile,
}

struct Brush {
    type_idx: usize,
    rotation: usize,
}

struct ExplorerApp {
    tiling: MarkedTiling<Label>,
    invalid_edges: HashSet<(Hex, usize)>,
    brush: Brush,
    mode: Mode,
    place_mode: PlaceMode,
    hover_hex: Option<Hex>,
    pan: egui::Vec2,
    zoom: f32,
    // Hex sets for each supertile produced by the last Supersubstitute.
    supertile_regions: Vec<HashSet<Hex>>,
    tracked: Option<TreeState>,
    // Generated-modes state (Marked hex & Spectre): the only ground truth is `tree_top`/`tree_path` — the
    // TreeCoords of the tile pinned at hex (0,0) with world rotation 0.  The
    // cache memoizes transducer-derived tiles (None = no tile there) and is
    // updated in place, not recomputed, when the context grows or shrinks.
    // `None` = empty canvas (nothing placed yet); `Some(t)` with an empty
    // path = the single unexpanded tile `t` at (0,0).
    tree_top: Option<u8>,
    tree_path: Vec<u8>,
    tree_cache: HashMap<Hex, Option<TreeTile>>,
    // Spectre-mode memo: placed spectre polygons keyed by (hexagon, index
    // within it).  Derived from the same tree state; the seed spectre of
    // hex (0,0) is pinned, so re-derivation is deterministic.
    spectre_cache: HashMap<(Hex, u8), [Zd; 14]>,
    // Spectre under the cursor (hit-tested in the spectre plane, where the
    // hex-grid hover is meaningless).
    hover_spectre: Option<(Hex, u8)>,
    show_borders: bool,
    show_names: bool,
    show_edge_labels: bool,
    show_paths: bool,
    show_colors: bool,
    border_palette: BorderPalette,
    edge_style: EdgeStyle,
}

impl Default for ExplorerApp {
    fn default() -> Self {
        Self {
            tiling: MarkedTiling::new(),
            invalid_edges: HashSet::new(),
            brush: Brush {
                type_idx: 0,
                rotation: 0,
            },
            mode: Mode::Spectre,
            place_mode: PlaceMode::Single,
            hover_hex: None,
            pan: egui::Vec2::ZERO,
            zoom: 50.0,
            supertile_regions: Vec::new(),
            tracked: None,
            tree_top: None,
            tree_path: Vec::new(),
            tree_cache: HashMap::new(),
            spectre_cache: HashMap::new(),
            hover_spectre: None,
            show_borders: true,
            show_names: true,
            show_edge_labels: false,
            show_paths: true,
            show_colors: true,
            border_palette: BorderPalette::Plain,
            edge_style: EdgeStyle::Tile11,
        }
    }
}

impl ExplorerApp {
    // Returns the patch to place (at origin), respecting current submode and brush.
    fn placement_patch(&self) -> MarkedTiling<Label> {
        match self.place_mode {
            PlaceMode::Single => {
                let mut t = MarkedTiling::new();
                t.insert(
                    Hex::new(0, 0),
                    BASE_TILES[self.brush.type_idx].rotate(self.brush.rotation),
                );
                t
            }
            PlaceMode::Supertile => rotate_tiling(
                &BASE_SUPERTILE_FNS[self.brush.type_idx](),
                self.brush.rotation,
            ),
        }
    }

    // TreeCoords of a brush patch just placed at `at` on an empty canvas:
    // a single tile is the (empty-path) top supertile itself; a base
    // supertile gives each child its one-step path.
    fn placed_tree_state(&self, at: Hex) -> TreeState {
        let mut coords = HashMap::new();
        match self.place_mode {
            PlaceMode::Single => {
                coords.insert(at, TreeCoords::new());
            }
            PlaceMode::Supertile => {
                for (i, ch) in SUPERTILE_CHILDREN[self.brush.type_idx].iter().enumerate() {
                    let mut h = ch.hex;
                    for _ in 0..self.brush.rotation {
                        h = h.rotate_cw();
                    }
                    coords.insert(
                        h + at,
                        TreeCoords {
                            path: vec![i as u8],
                        },
                    );
                }
            }
        }
        TreeState {
            top: self.brush.type_idx as u8,
            coords,
            via_transducer: false,
        }
    }

    // ---- Generated modes: the tiling generated from the (0,0) tile's TreeCoords ----

    // Place the root tile on an empty canvas: full path of one letter, the
    // tile itself pinned at (0,0).
    fn tree_set_root(&mut self, t: u8) {
        self.tree_top = Some(t);
        self.tree_path.clear();
        self.tree_cache.clear();
        self.spectre_cache.clear();
    }

    // Grow the context: declare the current top supertile to be child `i` of
    // a `parent`-typed supertile.  Every generated tile keeps its position
    // and rotation (the (0,0) pin is unchanged); cached paths just gain the
    // new leading step, while cached absences may now resolve and are
    // forgotten.
    fn tree_prepend(&mut self, parent: u8, i: u8) {
        debug_assert_eq!(
            Some(SUPERTILE_CHILDREN[parent as usize][i as usize].type_idx),
            self.tree_top,
        );
        self.tree_path.insert(0, i);
        self.tree_top = Some(parent);
        self.tree_cache.retain(|_, e| e.is_some());
        for tile in self.tree_cache.values_mut().flatten() {
            tile.coords.path.insert(0, i);
        }
        // Re-derived from the pinned seed; surviving spectres reappear in
        // the same places.
        self.spectre_cache.clear();
    }

    // Shrink the context: drop the leading step, keeping only the tiles of
    // the child supertile the (0,0) tile descends from.  Shrinking a single
    // letter empties the canvas.
    fn tree_strip(&mut self) {
        let Some(top) = self.tree_top else { return };
        if self.tree_path.is_empty() {
            self.tree_reset();
            return;
        }
        let c0 = self.tree_path.remove(0);
        self.tree_top = Some(SUPERTILE_CHILDREN[top as usize][c0 as usize].type_idx);
        self.tree_cache
            .retain(|_, e| matches!(e, Some(tile) if tile.coords.path.first() == Some(&c0)));
        for tile in self.tree_cache.values_mut().flatten() {
            tile.coords.path.remove(0);
        }
        self.spectre_cache.clear();
    }

    // Back to the empty canvas.
    fn tree_reset(&mut self) {
        self.tree_top = None;
        self.tree_path.clear();
        self.tree_cache.clear();
        self.spectre_cache.clear();
    }

    // Seed the tree cache with the pinned (0,0) tile if absent.
    fn tree_ensure_seed(&mut self) {
        let Some(top) = self.tree_top else { return };
        let seed = Hex::new(0, 0);
        if !self.tree_cache.contains_key(&seed) {
            self.tree_cache.insert(
                seed,
                Some(TreeTile {
                    coords: TreeCoords {
                        path: self.tree_path.clone(),
                    },
                    type_idx: *types_along(top, &self.tree_path).last().unwrap(),
                    rot: 0,
                }),
            );
        }
    }

    // (type, world rotation) of the hexagon across world direction `w` from
    // `from`, extending the tree cache via the transducer on a miss.
    fn tree_neighbor_tile(
        tree: &mut HashMap<Hex, Option<TreeTile>>,
        top: u8,
        from: Hex,
        w: usize,
    ) -> Option<(u8, u8)> {
        let nb = from + DIRECTIONS[w];
        if let Some(e) = tree.get(&nb) {
            return e.as_ref().map(|t| (t.type_idx, t.rot));
        }
        let tile = tree.get(&from)?.clone()?;
        let delta = ((w + 6 - tile.rot as usize) % 6) as u8;
        let entry = Transducer::global()
            .neighbor(top, &tile.coords, delta)
            .map(|(nc, back)| TreeTile {
                type_idx: *types_along(top, &nc.path).last().unwrap(),
                rot: ((w + 9 - back as usize) % 6) as u8,
                coords: nc,
            });
        let res = entry.as_ref().map(|t| (t.type_idx, t.rot));
        tree.insert(nb, entry);
        res
    }

    // Extend the generated picture over `rect`: walk hex adjacency with the
    // transducer outward from the already computed region (initially the
    // pinned (0,0) tile), memoizing both tiles and absences.  If nothing
    // computed is on screen, the walk is let in from the seed.
    fn tree_fill(&mut self, rect: egui::Rect, canvas_center: egui::Pos2) {
        let Some(top) = self.tree_top else { return };
        let t = Transducer::global();
        self.tree_ensure_seed();
        let seed = Hex::new(0, 0);
        let mut bound = rect.expand(4.0 * self.zoom);
        let mut queue: VecDeque<Hex> = VecDeque::new();
        for (&h, e) in &self.tree_cache {
            if e.is_some() && bound.contains(hex_to_screen(h, self.zoom, self.pan, canvas_center)) {
                queue.push_back(h);
            }
        }
        if queue.is_empty() {
            let seed_sc = hex_to_screen(seed, self.zoom, self.pan, canvas_center);
            bound = bound.union(egui::Rect::from_min_max(seed_sc, seed_sc).expand(self.zoom));
            queue.push_back(seed);
        }
        while let Some(hex) = queue.pop_front() {
            let Some(Some(tile)) = self.tree_cache.get(&hex).cloned() else {
                continue;
            };
            for (w, &dir) in DIRECTIONS.iter().enumerate() {
                let nb = hex + dir;
                if self.tree_cache.contains_key(&nb)
                    || !bound.contains(hex_to_screen(nb, self.zoom, self.pan, canvas_center))
                {
                    continue;
                }
                let delta = ((w + 6 - tile.rot as usize) % 6) as u8;
                let entry = t
                    .neighbor(top, &tile.coords, delta)
                    .map(|(nc, back)| TreeTile {
                        type_idx: *types_along(top, &nc.path).last().unwrap(),
                        rot: ((w + 9 - back as usize) % 6) as u8,
                        coords: nc,
                    });
                let found = entry.is_some();
                self.tree_cache.insert(nb, entry);
                if found {
                    queue.push_back(nb);
                }
            }
        }
    }

    // Extend the spectre patch over `rect` by BFS gluing from the pinned
    // seed spectre (#0 of the hexagon at (0,0), edge 0 fixed at the origin),
    // deriving hexagon adjacency from the tree cache (extended on demand by
    // the transducer).  The hex grid and the spectre plane have different
    // geometries, so coverage is bounded by spectre screen positions.
    fn spectre_fill(&mut self, rect: egui::Rect, canvas_center: egui::Pos2) {
        let Some(top) = self.tree_top else { return };
        self.tree_ensure_seed();
        let (zoom, pan) = (self.zoom, self.pan);
        let mut tree = std::mem::take(&mut self.tree_cache);

        let seed_key = (Hex::new(0, 0), 0u8);
        self.spectre_cache
            .entry(seed_key)
            .or_insert_with(|| spectre_place(Zd::ZERO, Zd::ONE, 0));

        let mut bound = rect.expand(4.0 * zoom);
        let mut queue: VecDeque<(Hex, u8)> = self
            .spectre_cache
            .iter()
            .filter(|(_, poly)| bound.contains(zd_screen(poly[0], zoom, pan, canvas_center)))
            .map(|(&k, _)| k)
            .collect();
        if queue.is_empty() {
            let sc = zd_screen(self.spectre_cache[&seed_key][0], zoom, pan, canvas_center);
            bound = bound.union(egui::Rect::from_min_max(sc, sc).expand(zoom));
            queue.push_back(seed_key);
        }

        while let Some((hex, idx)) = queue.pop_front() {
            let poly = self.spectre_cache[&(hex, idx)];
            let Some(Some(tile)) = tree.get(&hex).cloned() else {
                continue;
            };
            for edge in 0..14u8 {
                let step = spectre_step(hex, (tile.type_idx, tile.rot), idx, edge, &mut |f, w| {
                    Self::tree_neighbor_tile(&mut tree, top, f, w)
                });
                let Some((h2, i2, e2)) = step else { continue };
                if self.spectre_cache.contains_key(&(h2, i2)) {
                    continue;
                }
                let p2 = spectre_place(
                    poly[(edge as usize + 1) % 14],
                    poly[edge as usize],
                    e2 as usize,
                );
                if !bound.contains(zd_screen(p2[0], zoom, pan, canvas_center)) {
                    continue;
                }
                self.spectre_cache.insert((h2, i2), p2);
                queue.push_back((h2, i2));
            }
        }

        self.tree_cache = tree;
    }

    // Draw the placed spectres: filled via the per-style cached
    // triangulation (the outline is concave), outlined in the selected edge
    // style, named after their hexagon's type, with supertile borders of
    // every order on top (a spectre edge inherits the order of the hexagon
    // edge it crosses; Γ's internal pair edge and sibling borders stay
    // plain).
    fn draw_spectres(&self, painter: &egui::Painter, rect: egui::Rect, canvas_center: egui::Pos2) {
        let Some(top) = self.tree_top else { return };
        let t = Transducer::global();
        let cull = rect.expand(4.0 * self.zoom);
        let style = self.edge_style;
        let s = style.samples();
        let mut border_segs: Vec<(usize, Vec<egui::Pos2>)> = Vec::new();
        for (&(hex, idx), poly) in &self.spectre_cache {
            let pts: [egui::Pos2; 14] =
                std::array::from_fn(|i| zd_screen(poly[i], self.zoom, self.pan, canvas_center));
            if !pts.iter().any(|p| cull.contains(*p)) {
                continue;
            }
            let Some(Some(tile)) = self.tree_cache.get(&hex) else {
                continue;
            };
            let base = self.tile_fill(tile.type_idx as usize);
            let color = if idx == 1 {
                // Γ's second spectre: the rare odd-orientation partner.
                egui::Color32::from_rgb(
                    (base.r() as u32 * 3 / 5) as u8,
                    (base.g() as u32 * 3 / 5) as u8,
                    (base.b() as u32 * 3 / 5) as u8,
                )
            } else {
                base
            };
            let outline = spectre_outline(&pts, style);
            let mut mesh = egui::epaint::Mesh::default();
            for &p in &outline {
                mesh.colored_vertex(p, color);
            }
            for tri in style_triangles(style) {
                mesh.indices.extend_from_slice(tri);
            }
            painter.add(egui::Shape::mesh(mesh));
            painter.add(egui::Shape::closed_line(
                outline.clone(),
                egui::Stroke::new(1.5, egui::Color32::from_rgb(25, 25, 25)),
            ));

            let cx = pts.iter().map(|p| p.x).sum::<f32>() / 14.0;
            let cy = pts.iter().map(|p| p.y).sum::<f32>() / 14.0;
            if self.show_names && self.zoom > 18.0 {
                painter.text(
                    egui::pos2(cx, cy),
                    egui::Align2::CENTER_CENTER,
                    TILE_NAMES[tile.type_idx as usize],
                    egui::FontId::proportional(self.zoom * 0.6),
                    self.tile_text_color(),
                );
            }

            if self.show_paths && self.zoom > 18.0 {
                let mut label = coords_str(top, &tile.coords);
                if tile.type_idx == 0 {
                    // Which of Γ's pair, as a subscript on the leaf step.
                    label.push(if idx == 0 { '₀' } else { '₁' });
                }
                painter.text(
                    egui::pos2(cx, cy + self.zoom * 0.55),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::monospace(self.zoom * 0.28),
                    self.path_label_color(),
                );
            }

            // Each boundary edge carries the marked label of the hexagon
            // edge it lies on (Γ's internal pair edge carries none).
            if self.show_edge_labels && self.zoom > 35.0 {
                let smap = SPECMAP[tile.type_idx as usize];
                let base = &BASE_TILES[tile.type_idx as usize];
                let font_size = (self.zoom * 0.20).max(8.0);
                for edge in 0..14usize {
                    let me = smap[14 * idx as usize + edge];
                    if me.internal {
                        continue;
                    }
                    let delta = our_direction(me.hi as usize);
                    let (a, b) = (pts[edge], pts[(edge + 1) % 14]);
                    let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                    // Inward normal: interior is on the rot-90° side of
                    // every directed edge (constant winding, no mirrors).
                    let d = b - a;
                    let n = egui::vec2(-d.y, d.x) / d.length().max(1e-6);
                    painter.text(
                        mid + n * (self.zoom * 0.28),
                        egui::Align2::CENTER_CENTER,
                        label_str(base.edges[delta]),
                        egui::FontId::proportional(font_size),
                        egui::Color32::from_rgb(20, 20, 20),
                    );
                }
            }

            if self.show_borders {
                let smap = SPECMAP[tile.type_idx as usize];
                for edge in 0..14usize {
                    let me = smap[14 * idx as usize + edge];
                    if me.internal {
                        continue;
                    }
                    let delta = our_direction(me.hi as usize) as u8;
                    let order = t.border_order(top, &tile.coords, delta);
                    if order > 0 {
                        // The edge's stretch of the outline, curve and all.
                        let seg = (0..=s)
                            .map(|k| outline[(edge * s + k) % (14 * s)])
                            .collect();
                        border_segs.push((order, seg));
                    }
                }
            }
        }
        border_segs.sort_unstable_by_key(|&(order, _)| order);
        for (order, seg) in border_segs {
            let width = ((self.zoom * 0.045).max(1.5) * (order as f32).sqrt()).min(self.zoom * 0.4);
            painter.add(egui::Shape::line(
                seg,
                egui::Stroke::new(width, self.order_color(order)),
            ));
        }
    }

    // Generated-modes side-panel section: the context of the pinned (0,0) tile and
    // the buttons that grow or shrink it.
    fn context_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.heading("Context");
        ui.separator();
        match self.tree_top {
            Some(top) => {
                ui.label(format!("depth {}", self.tree_path.len()));
                let seed = TreeCoords {
                    path: self.tree_path.clone(),
                };
                ui.monospace(format!("(0,0): {}", coords_str(top, &seed)));
            }
            None => {
                ui.monospace("empty canvas");
            }
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let can_shrink = self.tree_top.is_some();
            if ui
                .add_enabled(can_shrink, egui::Button::new("Shrink"))
                .clicked()
            {
                self.tree_strip();
            }
            if ui
                .add_enabled(can_shrink, egui::Button::new("Reset"))
                .clicked()
            {
                self.tree_reset();
            }
        });

        ui.add_space(6.0);
        let grow_button = |ui: &mut egui::Ui, type_idx: u8, label: String| -> bool {
            let color = TILE_COLORS[type_idx as usize];
            let btn = egui::Button::new(egui::RichText::new(label).color(color).strong())
                .fill(egui::Color32::from_rgba_unmultiplied(
                    color.r(),
                    color.g(),
                    color.b(),
                    50,
                ))
                .min_size(egui::vec2(44.0, 26.0));
            ui.add(btn).clicked()
        };
        match self.tree_top {
            None => {
                // The degenerate grow step: pick the first letter of the
                // full path, i.e. the root tile itself.
                ui.label("Grow — place the root tile:");
                ui.horizontal_wrapped(|ui| {
                    for t in 0..9u8 {
                        if grow_button(ui, t, TILE_NAMES[t as usize].to_string()) {
                            self.tree_set_root(t);
                        }
                    }
                });
            }
            Some(top) => {
                ui.label("Grow — become child of:");
                ui.horizontal_wrapped(|ui| {
                    for parent in 0..9u8 {
                        for (i, ch) in SUPERTILE_CHILDREN[parent as usize].iter().enumerate() {
                            if ch.type_idx != top {
                                continue;
                            }
                            if grow_button(ui, parent, step_label(parent, i as u8)) {
                                self.tree_prepend(parent, i as u8);
                            }
                        }
                    }
                });
            }
        }
    }

    // Hex-tiling-mode side-panel sections: place submode, brush, rotation,
    // preview, and the edit actions on the stored tiling.
    fn brush_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.heading("Place");
        ui.separator();
        for &(label, pm) in &[
            ("Single", PlaceMode::Single),
            ("Supertile", PlaceMode::Supertile),
        ] {
            let selected = self.place_mode == pm;
            let fill = if selected {
                egui::Color32::from_rgb(80, 120, 200)
            } else {
                egui::Color32::from_rgba_unmultiplied(80, 120, 200, 40)
            };
            if ui
                .add(
                    egui::Button::new(label)
                        .fill(fill)
                        .min_size(egui::vec2(110.0, 24.0)),
                )
                .clicked()
            {
                self.place_mode = pm;
            }
        }

        ui.add_space(10.0);
        ui.heading("Tile Type");
        ui.separator();
        for i in 0..9 {
            let color = TILE_COLORS[i];
            let selected = self.brush.type_idx == i;
            let fill = if selected {
                color
            } else {
                egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 50)
            };
            let text_color = if selected {
                egui::Color32::WHITE
            } else {
                color
            };
            let btn = egui::Button::new(
                egui::RichText::new(TILE_NAMES[i])
                    .color(text_color)
                    .strong(),
            )
            .fill(fill)
            .min_size(egui::vec2(110.0, 26.0));
            if ui.add(btn).clicked() {
                self.brush.type_idx = i;
            }
        }

        ui.add_space(10.0);
        ui.heading("Rotation");
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("↺ CCW").clicked() {
                self.brush.rotation = (self.brush.rotation + 1) % 6;
            }
            if ui.button("↻ CW").clicked() {
                self.brush.rotation = (self.brush.rotation + 5) % 6;
            }
        });
        ui.label(format!("{}×60° CCW", self.brush.rotation));

        // Preview — single tile always; supertile shows "n tiles" note
        ui.add_space(6.0);
        let (preview_resp, preview_painter) =
            ui.allocate_painter(egui::vec2(90.0, 90.0), egui::Sense::hover());
        let pc = preview_resp.rect.center();
        let pz = 32.0_f32;
        let tile = BASE_TILES[self.brush.type_idx].rotate(self.brush.rotation);
        let color = TILE_COLORS[self.brush.type_idx];
        preview_painter.add(egui::Shape::convex_polygon(
            hex_corners(pc, pz),
            color,
            egui::Stroke::new(1.5, egui::Color32::BLACK),
        ));
        let name_galley = preview_painter.layout_no_wrap(
            TILE_NAMES[self.brush.type_idx].to_string(),
            egui::FontId::proportional(pz * 0.42),
            egui::Color32::WHITE,
        );
        let nsz = name_galley.size();
        let angle = -(self.brush.rotation as f32) * std::f32::consts::PI / 3.0;
        let (cos_a, sin_a) = (angle.cos(), angle.sin());
        let name_pos = egui::pos2(
            pc.x - (nsz.x / 2.0 * cos_a - nsz.y / 2.0 * sin_a),
            pc.y - (nsz.x / 2.0 * sin_a + nsz.y / 2.0 * cos_a),
        );
        let mut name_shape =
            egui::epaint::TextShape::new(name_pos, name_galley, egui::Color32::WHITE);
        name_shape.angle = angle;
        preview_painter.add(egui::Shape::Text(name_shape));
        for i in 0..6 {
            let [a, b] = edge_endpoints(pc, pz, i);
            let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let inset = egui::pos2(mid.x + (pc.x - mid.x) * 0.3, mid.y + (pc.y - mid.y) * 0.3);
            preview_painter.text(
                inset,
                egui::Align2::CENTER_CENTER,
                label_str(tile.edges[i]),
                egui::FontId::proportional(8.0),
                egui::Color32::WHITE,
            );
        }
        if self.place_mode == PlaceMode::Supertile {
            let n = BASE_SUPERTILE_FNS[self.brush.type_idx]().tiles.len();
            ui.monospace(format!("({n} tiles)"));
        }

        ui.add_space(10.0);
        ui.separator();
        if ui.button("Clear All").clicked() {
            self.tiling = MarkedTiling::new();
            self.invalid_edges.clear();
            self.supertile_regions.clear();
            self.tracked = None;
        }
        let can_substitute = !self.tiling.tiles.is_empty();
        if ui
            .add_enabled(can_substitute, egui::Button::new("Supersubstitute"))
            .clicked()
        {
            let (new_tiling, placements) = supersubstitute_with_placements(&self.tiling);
            // Re-derive every tile's TreeCoords from a single seed: child 0
            // of any expanded supertile sits at the placement offset (its
            // local (0,0)) with path = source path + 0.
            self.tracked = self.tracked.take().and_then(|tr| {
                let (&src, pl) = placements.iter().next()?;
                let mut seed = tr.coords.get(&src)?.clone();
                seed.push(0);
                let coords = transducer_coords(tr.top, pl.offset, seed, &new_tiling)?;
                Some(TreeState {
                    top: tr.top,
                    coords,
                    via_transducer: true,
                })
            });
            self.supertile_regions = placements.values().map(placement_cells).collect();
            self.tiling = new_tiling;
            self.invalid_edges = recompute_invalid(&self.tiling);
        }
        ui.add_space(6.0);
        let n = self.tiling.tiles.len();
        let bad = self.invalid_edges.len() / 2;
        if n == 0 {
            ui.label("Empty tiling");
        } else if bad == 0 {
            ui.colored_label(egui::Color32::from_rgb(50, 200, 80), "Valid tiling ✓");
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(220, 60, 40),
                format!("{bad} bad edge{}", if bad == 1 { "" } else { "s" }),
            );
        }
        ui.label(format!("{n} tile{}", if n == 1 { "" } else { "s" }));
    }

    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tools")
            .min_width(130.0)
            .show(ctx, |ui| {
                // Mode toggle
                ui.heading("Mode");
                ui.separator();
                for &(label, mode) in &[
                    ("Marked hex", Mode::MarkedHex),
                    ("Spectre", Mode::Spectre),
                    ("Hex tiling", Mode::HexTiling),
                ] {
                    let selected = self.mode == mode;
                    let fill = if selected {
                        egui::Color32::from_rgb(80, 120, 200)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(80, 120, 200, 40)
                    };
                    if ui
                        .add(
                            egui::Button::new(label)
                                .fill(fill)
                                .min_size(egui::vec2(110.0, 24.0)),
                        )
                        .clicked()
                    {
                        self.mode = mode;
                    }
                }

                if self.mode == Mode::HexTiling {
                    self.brush_controls(ui);
                } else {
                    self.context_controls(ui);
                }

                ui.add_space(8.0);
                ui.heading("View");
                ui.separator();
                ui.checkbox(&mut self.show_colors, "Tile colors");
                ui.checkbox(&mut self.show_names, "Tile names");
                ui.checkbox(&mut self.show_paths, "Tree paths");
                ui.checkbox(&mut self.show_edge_labels, "Edge labels");
                ui.checkbox(&mut self.show_borders, "Supertile borders");
                ui.horizontal(|ui| {
                    ui.label("Palette:");
                    egui::ComboBox::from_id_salt("border-palette")
                        .selected_text(self.border_palette.name())
                        .show_ui(ui, |ui| {
                            for p in BorderPalette::ALL {
                                ui.selectable_value(&mut self.border_palette, p, p.name());
                            }
                        });
                });
                if self.mode == Mode::Spectre {
                    // The three published tile shapes (paper Fig. 1.1):
                    // straight Tile(1,1) and two curved-edge Spectres.
                    ui.horizontal(|ui| {
                        ui.label("Edges:");
                        egui::ComboBox::from_id_salt("edge-style")
                            .selected_text(self.edge_style.name())
                            .show_ui(ui, |ui| {
                                for st in EdgeStyle::ALL {
                                    ui.selectable_value(&mut self.edge_style, st, st.name());
                                }
                            });
                    });
                }
                ui.add_space(4.0);
                if ui.button("Center (0,0)").clicked() {
                    self.pan = egui::Vec2::ZERO;
                }

                if self.mode == Mode::MarkedHex {
                    ui.add_space(8.0);
                    ui.heading("Tree Coordinates");
                    ui.separator();
                    let hovered = self
                        .hover_hex
                        .and_then(|h| self.tree_cache.get(&h))
                        .and_then(Option::as_ref);
                    match self.tree_top.zip(hovered) {
                        Some((top, tile)) => {
                            ui.monospace(coords_str(top, &tile.coords));
                        }
                        None => {
                            ui.monospace("hover a tile");
                        }
                    }
                } else if self.mode == Mode::Spectre {
                    ui.add_space(8.0);
                    ui.heading("Tree Coordinates");
                    ui.separator();
                    let hovered = self.hover_spectre.and_then(|(hex, idx)| {
                        self.tree_cache
                            .get(&hex)
                            .and_then(Option::as_ref)
                            .map(|tile| (tile, idx))
                    });
                    match self.tree_top.zip(hovered) {
                        Some((top, (tile, idx))) => {
                            let mut s = coords_str(top, &tile.coords);
                            if tile.type_idx == 0 {
                                // Which of Γ's pair, as a subscript on the leaf step.
                                s.push(if idx == 0 { '₀' } else { '₁' });
                            }
                            ui.monospace(s);
                        }
                        None => {
                            ui.monospace("hover a tile");
                        }
                    }
                } else if self.mode == Mode::HexTiling {
                    ui.add_space(8.0);
                    ui.heading("Tree Coordinates");
                    ui.separator();
                    match &self.tracked {
                        Some(tr) => {
                            let depth = tr.coords.values().next().map_or(0, TreeCoords::depth);
                            ui.label(format!(
                                "{} root, depth {depth}",
                                TILE_NAMES[tr.top as usize]
                            ));
                            if tr.via_transducer {
                                ui.monospace("recomputed via transducer");
                            }
                            match self.hover_hex.and_then(|h| tr.coords.get(&h)) {
                                Some(c) => {
                                    ui.monospace(coords_str(tr.top, c));
                                }
                                None => {
                                    ui.monospace("hover a tile");
                                }
                            }
                        }
                        None => {
                            ui.monospace(
                                "untracked — place one tile or supertile on an empty canvas",
                            );
                        }
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                if self.mode == Mode::HexTiling {
                    ui.monospace("Left-click: place");
                    ui.monospace("Right-click: erase");
                    ui.monospace("Drag: pan");
                    ui.monospace("Scroll: zoom");
                    ui.monospace("Q / E: rotate CCW / CW");
                } else {
                    ui.monospace("Drag: pan");
                    ui.monospace("Scroll: zoom");
                }
            });
    }

    // Fill + name + optional path label for a tile of `type_idx` rotated by
    // `rotation` at screen position `sc`.
    // Tile fill: the type color, or a plain paper-like tone when colors are
    // switched off.
    fn tile_fill(&self, type_idx: usize) -> egui::Color32 {
        if self.show_colors {
            TILE_COLORS[type_idx]
        } else {
            egui::Color32::from_rgb(0xd8, 0xd4, 0xc8)
        }
    }

    // Text drawn over tile fills: white over colors, dark over plain.
    fn tile_text_color(&self) -> egui::Color32 {
        if self.show_colors {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(40, 40, 40)
        }
    }

    fn path_label_color(&self) -> egui::Color32 {
        if self.show_colors {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 210)
        } else {
            egui::Color32::from_rgba_unmultiplied(30, 30, 30, 230)
        }
    }

    // Supertile border color for one order, per the selected palette.
    fn order_color(&self, order: usize) -> egui::Color32 {
        match self.border_palette.colors() {
            Some(palette) => palette[(order - 1).min(palette.len() - 1)],
            None if self.show_colors => egui::Color32::WHITE,
            None => egui::Color32::from_rgb(25, 25, 25),
        }
    }

    fn paint_tile(
        &self,
        painter: &egui::Painter,
        sc: egui::Pos2,
        type_idx: usize,
        rotation: usize,
        path_label: Option<&str>,
    ) {
        painter.add(egui::Shape::convex_polygon(
            hex_corners(sc, self.zoom),
            self.tile_fill(type_idx),
            egui::Stroke::new(1.5, egui::Color32::from_rgb(25, 25, 25)),
        ));
        if self.show_names && self.zoom > 25.0 {
            let galley = painter.layout_no_wrap(
                TILE_NAMES[type_idx].to_string(),
                egui::FontId::proportional(self.zoom * 0.32),
                self.tile_text_color(),
            );
            let sz = galley.size();
            // CCW visual rotation: negative angle in egui's CW-positive convention.
            // Each rotation step = 60° CCW.
            let angle = -(rotation as f32) * std::f32::consts::PI / 3.0;
            let (cos_a, sin_a) = (angle.cos(), angle.sin());
            // Place pos (top-left pivot) so the text center lands at sc after rotation.
            // Rotation matrix (CW-positive, y-down): x'=x*cos-y*sin, y'=x*sin+y*cos
            let pos = egui::pos2(
                sc.x - (sz.x / 2.0 * cos_a - sz.y / 2.0 * sin_a),
                sc.y - (sz.x / 2.0 * sin_a + sz.y / 2.0 * cos_a),
            );
            let mut text_shape = egui::epaint::TextShape::new(pos, galley, self.tile_text_color());
            text_shape.angle = angle;
            painter.add(egui::Shape::Text(text_shape));
        }
        if let Some(label) = path_label {
            painter.text(
                sc + egui::vec2(0.0, self.zoom * 0.45),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::monospace(self.zoom * 0.16),
                self.path_label_color(),
            );
        }
    }

    fn paint_empty(&self, painter: &egui::Painter, sc: egui::Pos2) {
        painter.add(egui::Shape::closed_line(
            hex_corners(sc, self.zoom),
            egui::Stroke::new(
                0.5,
                egui::Color32::from_rgba_unmultiplied(180, 180, 180, 60),
            ),
        ));
    }

    fn draw_hex(&self, painter: &egui::Painter, hex: Hex, sc: egui::Pos2) {
        if let Some(tile) = self.tiling.tiles.get(&hex) {
            let (type_idx, rotation) = tile_id(tile).unwrap_or((0, 0));
            let label = if self.show_paths && self.zoom > 35.0 {
                self.tracked
                    .as_ref()
                    .and_then(|tr| tr.coords.get(&hex).map(|c| coords_str(tr.top, c)))
            } else {
                None
            };
            self.paint_tile(painter, sc, type_idx, rotation, label.as_deref());
        } else {
            self.paint_empty(painter, sc);
        }
    }

    fn draw_invalid_edges(&self, painter: &egui::Painter, hex: Hex, sc: egui::Pos2) {
        for i in 0..6 {
            if self.invalid_edges.contains(&(hex, i)) {
                let [a, b] = edge_endpoints(sc, self.zoom, i);
                painter.line_segment(
                    [a, b],
                    egui::Stroke::new(
                        (self.zoom * 0.13).max(2.5),
                        egui::Color32::from_rgb(255, 70, 0),
                    ),
                );
            }
        }
    }

    fn draw_edge_labels(
        painter: &egui::Painter,
        tile: &MarkedTile<Label>,
        sc: egui::Pos2,
        zoom: f32,
    ) {
        let font_size = (zoom * 0.20).max(8.0);
        for i in 0..6 {
            let [a, b] = edge_endpoints(sc, zoom, i);
            let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let inset = egui::pos2(mid.x + (sc.x - mid.x) * 0.25, mid.y + (sc.y - mid.y) * 0.25);
            painter.text(
                inset,
                egui::Align2::CENTER_CENTER,
                label_str(tile.edges[i]),
                egui::FontId::proportional(font_size),
                egui::Color32::from_rgb(20, 20, 20),
            );
        }
    }

    // Supertile borders of every order, found by the transducer: the order
    // of the boundary crossed by a move is the number of levels its carry
    // stays unresolved (`Transducer::border_order`; 0 = plain tile border
    // between siblings, already drawn by the tiles; full depth = the outer
    // rim of the patch).  `lookup` says which tile (coords + world rotation)
    // sits at a hex.  Thickness grows with the square root of the order;
    // higher orders are drawn last so they paint over lower ones.
    fn draw_order_borders(
        &self,
        painter: &egui::Painter,
        hexes: &[Hex],
        canvas_center: egui::Pos2,
        top: u8,
        lookup: &dyn Fn(Hex) -> Option<(TreeCoords, u8)>,
    ) {
        let t = Transducer::global();
        let mut segments: Vec<(usize, [egui::Pos2; 2])> = Vec::new();
        for &hex in hexes {
            let Some((coords, rho)) = lookup(hex) else {
                continue;
            };
            let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
            for (w, &dir) in DIRECTIONS.iter().enumerate() {
                // Each interior edge once (from its E/NE/NW side); rim edges
                // are always drawn from the tile side.
                if w >= 3 && lookup(hex + dir).is_some() {
                    continue;
                }
                let delta = ((w + 6 - rho as usize) % 6) as u8;
                let order = t.border_order(top, &coords, delta);
                if order > 0 {
                    segments.push((order, edge_endpoints(sc, self.zoom, w)));
                }
            }
        }
        segments.sort_unstable_by_key(|&(order, _)| order);
        for (order, seg) in segments {
            let width = ((self.zoom * 0.045).max(1.5) * (order as f32).sqrt()).min(self.zoom * 0.4);
            painter.line_segment(seg, egui::Stroke::new(width, self.order_color(order)));
        }
    }

    fn draw_supertile_outlines(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        canvas_center: egui::Pos2,
    ) {
        let stroke = egui::Stroke::new((self.zoom * 0.08).max(2.0), self.order_color(1));
        let cull_rect = rect.expand(self.zoom * 2.0);
        for region in &self.supertile_regions {
            for &hex in region {
                let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                if !cull_rect.contains(sc) {
                    continue;
                }
                for (i, &dir) in DIRECTIONS.iter().enumerate() {
                    if !region.contains(&(hex + dir)) {
                        let [a, b] = edge_endpoints(sc, self.zoom, i);
                        painter.line_segment([a, b], stroke);
                    }
                }
            }
        }
    }

    fn draw_ghost(
        painter: &egui::Painter,
        patch: &MarkedTiling<Label>,
        offset: Hex,
        zoom: f32,
        pan: egui::Vec2,
        canvas_center: egui::Pos2,
    ) {
        for (&h, tile) in &patch.tiles {
            let sc = hex_to_screen(h + offset, zoom, pan, canvas_center);
            let (type_idx, rotation) = tile_id(tile).unwrap_or((0, 0));
            let c = TILE_COLORS[type_idx];
            painter.add(egui::Shape::convex_polygon(
                hex_corners(sc, zoom),
                egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 120),
                egui::Stroke::new(
                    1.5,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 140),
                ),
            ));
            if zoom > 25.0 {
                let galley = painter.layout_no_wrap(
                    TILE_NAMES[type_idx].to_string(),
                    egui::FontId::proportional(zoom * 0.32),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                );
                let sz = galley.size();
                let angle = -(rotation as f32) * std::f32::consts::PI / 3.0;
                let (cos_a, sin_a) = (angle.cos(), angle.sin());
                let pos = egui::pos2(
                    sc.x - (sz.x / 2.0 * cos_a - sz.y / 2.0 * sin_a),
                    sc.y - (sz.x / 2.0 * sin_a + sz.y / 2.0 * cos_a),
                );
                let mut text_shape = egui::epaint::TextShape::new(
                    pos,
                    galley,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                );
                text_shape.angle = angle;
                painter.add(egui::Shape::Text(text_shape));
            }
            if zoom > 35.0 {
                let font_size = (zoom * 0.20).max(8.0);
                for i in 0..6 {
                    let [a, b] = edge_endpoints(sc, zoom, i);
                    let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                    let inset =
                        egui::pos2(mid.x + (sc.x - mid.x) * 0.25, mid.y + (sc.y - mid.y) * 0.25);
                    painter.text(
                        inset,
                        egui::Align2::CENTER_CENTER,
                        label_str(tile.edges[i]),
                        egui::FontId::proportional(font_size),
                        egui::Color32::from_rgba_unmultiplied(20, 20, 20, 200),
                    );
                }
            }
        }
    }

    fn handle_input(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        canvas_center: egui::Pos2,
    ) {
        // Track hover hex for ghost preview
        self.hover_hex = response
            .hover_pos()
            .map(|pos| screen_to_hex(pos, self.zoom, self.pan, canvas_center));

        // Keyboard rotation: Q = CCW, E = CW
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Q) {
                self.brush.rotation = (self.brush.rotation + 1) % 6;
            }
            if i.key_pressed(egui::Key::E) {
                self.brush.rotation = (self.brush.rotation + 5) % 6;
            }
        });

        // Pan via drag
        self.pan += response.drag_delta();

        // Zoom toward cursor
        let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_y != 0.0 {
            let factor = (1.0 + scroll_y * 0.002).clamp(0.85, 1.18);
            let old_zoom = self.zoom;
            let old_pan = self.pan;
            self.zoom = (old_zoom * factor).clamp(10.0, 300.0);
            if let Some(hover) = response.hover_pos() {
                let actual_factor = self.zoom / old_zoom;
                self.pan = old_pan + (hover - canvas_center - old_pan) * (1.0 - actual_factor);
            }
        }

        // Marked-hex and Spectre modes are read-only: the tiling is generated,
        // not edited.
        if self.mode != Mode::HexTiling {
            return;
        }

        // Place on left click
        if response.clicked_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
        {
            let hex = screen_to_hex(pos, self.zoom, self.pan, canvas_center);
            let was_empty = self.tiling.tiles.is_empty();
            let patch = self.placement_patch();
            for (&h, tile) in &patch.tiles {
                self.tiling.insert(h + hex, tile.clone());
            }
            self.invalid_edges = recompute_invalid(&self.tiling);
            // A patch on an empty canvas roots a fresh hierarchy; any
            // other placement is free-form and orphans the coords.
            self.tracked = was_empty.then(|| self.placed_tree_state(hex));
        }

        // Erase single tile on right click
        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let hex = screen_to_hex(pos, self.zoom, self.pan, canvas_center);
            if self.tiling.tiles.remove(&hex).is_some() {
                self.tracked = None;
            }
            self.invalid_edges = recompute_invalid(&self.tiling);
        }
    }
}

impl eframe::App for ExplorerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.side_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = response.rect;
            let canvas_center = rect.center();

            self.handle_input(ctx, &response, canvas_center);

            let hexes = visible_hexes(rect, self.zoom, self.pan);

            if self.mode == Mode::MarkedHex {
                self.tree_fill(rect, canvas_center);

                for &hex in &hexes {
                    let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                    match self.tree_cache.get(&hex).and_then(Option::as_ref) {
                        Some(tile) => {
                            let label = if self.show_paths && self.zoom > 35.0 {
                                self.tree_top.map(|top| coords_str(top, &tile.coords))
                            } else {
                                None
                            };
                            self.paint_tile(
                                &painter,
                                sc,
                                tile.type_idx as usize,
                                tile.rot as usize,
                                label.as_deref(),
                            );
                        }
                        None => self.paint_empty(&painter, sc),
                    }
                }

                if self.show_edge_labels && self.zoom > 35.0 {
                    for &hex in &hexes {
                        if let Some(Some(tile)) = self.tree_cache.get(&hex) {
                            let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                            let mt = BASE_TILES[tile.type_idx as usize].rotate(tile.rot as usize);
                            Self::draw_edge_labels(&painter, &mt, sc, self.zoom);
                        }
                    }
                }

                if let (true, Some(top)) = (self.show_borders, self.tree_top) {
                    self.draw_order_borders(&painter, &hexes, canvas_center, top, &|h| {
                        self.tree_cache
                            .get(&h)?
                            .as_ref()
                            .map(|tile| (tile.coords.clone(), tile.rot))
                    });
                }
            } else if self.mode == Mode::Spectre {
                self.spectre_fill(rect, canvas_center);
                self.hover_spectre = response.hover_pos().and_then(|pos| {
                    self.spectre_cache.iter().find_map(|(&key, poly)| {
                        // Cheap pre-filter: the whole tile is within ~5
                        // edge units of vertex 0.
                        let v0 = zd_screen(poly[0], self.zoom, self.pan, canvas_center);
                        if v0.distance(pos) > 6.0 * self.zoom {
                            return None;
                        }
                        let pts: Vec<egui::Pos2> = poly
                            .iter()
                            .map(|&p| zd_screen(p, self.zoom, self.pan, canvas_center))
                            .collect();
                        point_in_polygon(pos, &pts).then_some(key)
                    })
                });
                self.draw_spectres(&painter, rect, canvas_center);
            } else {
                for &hex in &hexes {
                    let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                    self.draw_hex(&painter, hex, sc);
                }

                if self.show_edge_labels && self.zoom > 35.0 {
                    for &hex in &hexes {
                        if let Some(tile) = self.tiling.tiles.get(&hex).cloned() {
                            let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                            Self::draw_edge_labels(&painter, &tile, sc, self.zoom);
                        }
                    }
                }

                // Supertile borders: order-by-thickness from TreeCoords when
                // tracked, else the flat outlines of the last Supersubstitute.
                if self.show_borders {
                    if let Some(tr) = &self.tracked {
                        self.draw_order_borders(&painter, &hexes, canvas_center, tr.top, &|h| {
                            let c = tr.coords.get(&h)?;
                            let (_, rho) = self.tiling.tiles.get(&h).and_then(tile_id)?;
                            Some((c.clone(), rho as u8))
                        });
                    } else if !self.supertile_regions.is_empty() {
                        self.draw_supertile_outlines(&painter, rect, canvas_center);
                    }
                }

                // Invalid edges drawn last so red bad edges paint over supertile outlines.
                for &hex in &hexes {
                    let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                    self.draw_invalid_edges(&painter, hex, sc);
                }

                // Ghost preview of pending placement at hover position
                if let Some(hover) = self.hover_hex {
                    let patch = self.placement_patch();
                    Self::draw_ghost(&painter, &patch, hover, self.zoom, self.pan, canvas_center);
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Spectre Explorer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 800.0])
                .with_title("Spectre Explorer"),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(ExplorerApp::default()))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectre_tiling::tree_coords::{path_rotation, types_along};

    /// Every tile has coords whose leaf type matches the tile, and whose
    /// rotation differs from the tile's world rotation by one global constant
    /// (the patch's rigid rotation).
    fn assert_coords_match(
        top: u8,
        tiling: &MarkedTiling<Label>,
        coords: &HashMap<Hex, TreeCoords>,
    ) {
        assert_eq!(coords.len(), tiling.tiles.len());
        let mut global_rot = None;
        for (hex, c) in coords {
            let (ti, rho) = tile_id(&tiling.tiles[hex]).unwrap();
            assert_eq!(
                *types_along(top, &c.path).last().unwrap() as usize,
                ti,
                "leaf type mismatch at {hex:?}",
            );
            let g = (rho + 6 - path_rotation(top, &c.path) as usize) % 6;
            assert_eq!(
                *global_rot.get_or_insert(g),
                g,
                "inconsistent global rotation at {hex:?}",
            );
        }
    }

    /// The app flow: place a rotated supertile on an empty canvas, then
    /// Supersubstitute twice, each time re-deriving every tile's TreeCoords
    /// from a single seed via the transducer walk.
    #[test]
    fn transducer_walk_tracks_app_supersubstitution() {
        for top in [0usize, 6] {
            for rot in [0usize, 2] {
                let mut app = ExplorerApp {
                    mode: Mode::HexTiling,
                    place_mode: PlaceMode::Supertile,
                    brush: Brush {
                        type_idx: top,
                        rotation: rot,
                    },
                    ..Default::default()
                };
                let at = Hex::new(3, -2);
                let patch = app.placement_patch();
                for (&h, tile) in &patch.tiles {
                    app.tiling.insert(h + at, tile.clone());
                }
                let mut tree = app.placed_tree_state(at);
                let mut tiling = app.tiling;
                assert_coords_match(tree.top, &tiling, &tree.coords);

                for _ in 0..2 {
                    let (next, placements) = supersubstitute_with_placements(&tiling);
                    let (&src, pl) = placements.iter().next().unwrap();
                    let mut seed = tree.coords[&src].clone();
                    seed.push(0);
                    tree.coords = transducer_coords(tree.top, pl.offset, seed, &next)
                        .expect("transducer walk failed to cover the patch");
                    tiling = next;
                    assert_coords_match(tree.top, &tiling, &tree.coords);
                }
            }
        }
    }

    /// Marked-hex mode: growing the context around the pinned (0,0) tile generates
    /// exactly the corresponding supertile patches (the pinned embedding
    /// coincides with the canonical one at level 1), the depth-2 patch glues
    /// validly everywhere, and shrinking restores the previous states.
    #[test]
    fn tree_mode_generates_patches() {
        let mut app = ExplorerApp {
            zoom: 20.0,
            ..Default::default()
        };
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1280.0, 800.0));
        let some_count =
            |app: &ExplorerApp| app.tree_cache.values().filter(|e| e.is_some()).count();

        // The canvas starts empty; placing the root Γ is the first step.
        assert_eq!(app.tree_top, None);
        app.tree_fill(rect, rect.center());
        assert_eq!(some_count(&app), 0);
        app.tree_set_root(0);
        app.tree_fill(rect, rect.center());
        assert_eq!(some_count(&app), 1);

        // Γ is child 0 of Δ; the level-1 patch must equal the Δ supertile.
        app.tree_prepend(1, 0);
        app.tree_fill(rect, rect.center());
        let expected = supertile_delta();
        assert_eq!(some_count(&app), expected.tiles.len());
        for (&hex, want) in &expected.tiles {
            let got = app.tree_cache[&hex].as_ref().unwrap();
            assert_eq!(
                (got.type_idx as usize, got.rot as usize),
                tile_id(want).unwrap(),
                "at {hex:?}",
            );
        }

        // One more level: Δ is child 5 of Γ; the depth-2 patch has 55 tiles
        // (Γ expands to 7 supertiles, one of which is the 7-tile Γ).
        app.tree_prepend(0, 5);
        app.tree_fill(rect, rect.center());
        let mut tiling = MarkedTiling::new();
        for (&h, e) in &app.tree_cache {
            if let Some(tile) = e {
                tiling.insert(
                    h,
                    BASE_TILES[tile.type_idx as usize].rotate(tile.rot as usize),
                );
            }
        }
        assert_eq!(tiling.tiles.len(), 55);
        assert!(tiling.is_valid(), "generated depth-2 patch has bad edges");

        // Shrinking twice returns to the single pinned tile, a third time
        // back to the empty canvas.
        app.tree_strip();
        assert_eq!(
            (app.tree_top, app.tree_path.as_slice()),
            (Some(1), &[0u8][..])
        );
        assert_eq!(some_count(&app), 8);
        app.tree_strip();
        assert_eq!((app.tree_top, app.tree_path.len()), (Some(0), 0));
        assert_eq!(some_count(&app), 1);
        app.tree_strip();
        assert_eq!(app.tree_top, None);
        assert_eq!(app.tree_cache.len(), 0);
    }

    /// Every edge style's curve is odd around the edge midpoint, so the two
    /// tiles sharing an edge trace the same arc and the styles glue without
    /// gaps or overlaps.
    #[test]
    fn edge_profiles_glue_both_ways() {
        for style in EdgeStyle::ALL {
            for k in 0..=24 {
                let t = k as f32 / 24.0;
                assert!(
                    (style.offset(t) + style.offset(1.0 - t)).abs() < 1e-5,
                    "{style:?} not point-symmetric at t = {t}",
                );
            }
        }
    }

    /// Each style's cached triangulation covers its outline exactly: n − 2
    /// triangles whose unsigned areas sum to the polygon area (overlapping
    /// or escaping triangles would inflate the sum).
    #[test]
    fn style_triangulations_fill_the_outline() {
        let poly = spectre_place(Zd::ZERO, Zd::ONE, 0);
        let pts: [egui::Pos2; 14] = std::array::from_fn(|i| {
            let (x, y) = poly[i].to_xy();
            egui::pos2(x as f32, -y as f32)
        });
        for style in EdgeStyle::ALL {
            let outline = spectre_outline(&pts, style);
            let tris = style_triangles(style);
            assert_eq!(tris.len(), outline.len() - 2, "{style:?}: triangle count");
            let polygon_area: f32 = (0..outline.len())
                .map(|i| {
                    let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
                    (a.x * b.y - b.x * a.y) / 2.0
                })
                .sum();
            let triangle_area: f32 = tris
                .iter()
                .map(|tri| {
                    let [a, b, c] = tri.map(|i| outline[i as usize]);
                    ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() / 2.0
                })
                .sum();
            assert!(
                (polygon_area.abs() - triangle_area).abs() < polygon_area.abs() * 1e-3,
                "{style:?}: polygon area {polygon_area} vs triangulated {triangle_area}",
            );
        }
    }

    /// Spectre mode: the depth-2 patch yields one spectre per hexagon plus
    /// one extra per Γ, all glued from the pinned seed.
    #[test]
    fn spectre_mode_generates_patch() {
        let mut app = ExplorerApp {
            zoom: 20.0,
            mode: Mode::Spectre,
            ..Default::default()
        };
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1280.0, 800.0));
        app.tree_set_root(0); // a lone Γ
        app.tree_prepend(1, 0); // Γ → child 0 of Δ
        app.tree_prepend(0, 5); // Δ → child 5 of Γ
        app.spectre_fill(rect, rect.center());

        let tiles: Vec<_> = app.tree_cache.values().flatten().collect();
        assert_eq!(tiles.len(), 55);
        let gammas = tiles.iter().filter(|t| t.type_idx == 0).count();
        assert_eq!(gammas, 7);
        assert_eq!(app.spectre_cache.len(), 55 + gammas);

        // Every spectre belongs to a present hexagon, and every Γ hexagon
        // carries exactly its mystic pair.
        for &(hex, idx) in app.spectre_cache.keys() {
            let tile = app.tree_cache[&hex].as_ref().unwrap();
            assert!(idx < if tile.type_idx == 0 { 2 } else { 1 });
        }
    }
}
