use std::collections::{HashMap, HashSet, VecDeque};

use eframe::egui;
use spectre_tiling::hex::{Hex, DIRECTIONS};
use spectre_tiling::marked::{MarkedTile, MarkedTiling};
use spectre_tiling::spectre::Label;
use spectre_tiling::supertile::{
    supertile_delta, supertile_gamma, supertile_lambda, supertile_phi, supertile_pi,
    supertile_psi, supertile_sigma, supertile_theta, supertile_xi,
};
use spectre_tiling::tiling::{
    placement_cells, supersubstitute_with_placements, tile_id, BASE_TILES, TILE_NAMES,
};
use spectre_tiling::transducer::Transducer;
use spectre_tiling::tree_coords::{types_along, TreeCoords, SUPERTILE_CHILDREN};

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
    [corner(sc, zoom, (11 - i) % 6), corner(sc, zoom, (6 - i) % 6)]
}

fn hex_to_screen(hex: Hex, zoom: f32, pan: egui::Vec2, canvas_center: egui::Pos2) -> egui::Pos2 {
    let sqrt3 = 3f32.sqrt();
    let wx = sqrt3 * hex.q as f32 + sqrt3 / 2.0 * hex.r as f32;
    let wy = -1.5 * hex.r as f32;
    canvas_center + pan + egui::vec2(wx * zoom, wy * zoom)
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

// Path as the tile name of the child picked at each level (root-first), with
// the child index as a superscript — names alone are ambiguous because a
// supertile can contain two children of the same type; "ε" for the root.
fn coords_str(top: u8, c: &TreeCoords) -> String {
    const SUP: [char; 8] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷'];
    if c.path.is_empty() {
        return "ε".to_string();
    }
    let types = types_along(top, &c.path);
    c.path
        .iter()
        .zip(&types[1..])
        .map(|(&i, &t)| format!("{}{}", TILE_NAMES[t as usize], SUP[i as usize]))
        .collect()
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
enum PlaceMode {
    Single,
    Supertile,
}

struct Brush {
    type_idx: usize,
    rotation: usize,
}

struct HexApp {
    tiling: MarkedTiling<Label>,
    invalid_edges: HashSet<(Hex, usize)>,
    brush: Brush,
    mode: PlaceMode,
    hover_hex: Option<Hex>,
    pan: egui::Vec2,
    zoom: f32,
    // Hex sets for each supertile produced by the last Supersubstitute.
    supertile_regions: Vec<HashSet<Hex>>,
    tree: Option<TreeState>,
    show_borders: bool,
    show_names: bool,
    show_edge_labels: bool,
    show_paths: bool,
}

impl Default for HexApp {
    fn default() -> Self {
        Self {
            tiling: MarkedTiling::new(),
            invalid_edges: HashSet::new(),
            brush: Brush {
                type_idx: 0,
                rotation: 0,
            },
            mode: PlaceMode::Single,
            hover_hex: None,
            pan: egui::Vec2::ZERO,
            zoom: 50.0,
            supertile_regions: Vec::new(),
            tree: None,
            show_borders: true,
            show_names: true,
            show_edge_labels: false,
            show_paths: true,
        }
    }
}

impl HexApp {
    // Returns the patch to place (at origin), respecting current mode and brush.
    fn placement_patch(&self) -> MarkedTiling<Label> {
        match self.mode {
            PlaceMode::Single => {
                let mut t = MarkedTiling::new();
                t.insert(
                    Hex::new(0, 0),
                    BASE_TILES[self.brush.type_idx].rotate(self.brush.rotation),
                );
                t
            }
            PlaceMode::Supertile => {
                rotate_tiling(&BASE_SUPERTILE_FNS[self.brush.type_idx](), self.brush.rotation)
            }
        }
    }

    // TreeCoords of a brush patch just placed at `at` on an empty canvas:
    // a single tile is the (empty-path) top supertile itself; a base
    // supertile gives each child its one-step path.
    fn placed_tree_state(&self, at: Hex) -> TreeState {
        let mut coords = HashMap::new();
        match self.mode {
            PlaceMode::Single => {
                coords.insert(at, TreeCoords::new());
            }
            PlaceMode::Supertile => {
                for (i, ch) in SUPERTILE_CHILDREN[self.brush.type_idx].iter().enumerate() {
                    let mut h = ch.hex;
                    for _ in 0..self.brush.rotation {
                        h = h.rotate_cw();
                    }
                    coords.insert(h + at, TreeCoords { path: vec![i as u8] });
                }
            }
        }
        TreeState {
            top: self.brush.type_idx as u8,
            coords,
            via_transducer: false,
        }
    }

    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tools").min_width(130.0).show(ctx, |ui| {
            // Mode toggle
            ui.heading("Mode");
            ui.separator();
            for &(label, mode) in &[
                ("Single", PlaceMode::Single),
                ("Supertile", PlaceMode::Supertile),
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
                let text_color = if selected { egui::Color32::WHITE } else { color };
                let btn = egui::Button::new(
                    egui::RichText::new(TILE_NAMES[i]).color(text_color).strong(),
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
                let inset = egui::pos2(
                    mid.x + (pc.x - mid.x) * 0.3,
                    mid.y + (pc.y - mid.y) * 0.3,
                );
                preview_painter.text(
                    inset,
                    egui::Align2::CENTER_CENTER,
                    label_str(tile.edges[i]),
                    egui::FontId::proportional(8.0),
                    egui::Color32::WHITE,
                );
            }
            if self.mode == PlaceMode::Supertile {
                let n = BASE_SUPERTILE_FNS[self.brush.type_idx]().tiles.len();
                ui.small(format!("({n} tiles)"));
            }

            ui.add_space(10.0);
            ui.separator();
            if ui.button("Clear All").clicked() {
                self.tiling = MarkedTiling::new();
                self.invalid_edges.clear();
                self.supertile_regions.clear();
                self.tree = None;
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
                self.tree = self.tree.take().and_then(|tree| {
                    let (&src, pl) = placements.iter().next()?;
                    let mut seed = tree.coords.get(&src)?.clone();
                    seed.push(0);
                    let coords = transducer_coords(tree.top, pl.offset, seed, &new_tiling)?;
                    Some(TreeState { top: tree.top, coords, via_transducer: true })
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

            ui.add_space(8.0);
            ui.heading("View");
            ui.separator();
            ui.checkbox(&mut self.show_names, "Tile names");
            ui.checkbox(&mut self.show_paths, "Tree paths");
            ui.checkbox(&mut self.show_edge_labels, "Edge labels");
            ui.checkbox(&mut self.show_borders, "Supertile borders");

            ui.add_space(8.0);
            ui.heading("TreeCoords");
            ui.separator();
            match &self.tree {
                Some(tree) => {
                    let depth = tree.coords.values().next().map_or(0, TreeCoords::depth);
                    ui.label(format!("{} root, depth {depth}", TILE_NAMES[tree.top as usize]));
                    if tree.via_transducer {
                        ui.small("recomputed via transducer");
                    }
                    match self.hover_hex.and_then(|h| tree.coords.get(&h)) {
                        Some(c) => {
                            ui.monospace(format!("path {}", coords_str(tree.top, c)));
                        }
                        None => {
                            ui.small("hover a tile for its path");
                        }
                    }
                }
                None => {
                    ui.small("untracked — place one tile or supertile on an empty canvas");
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.small("Left-click: place");
            ui.small("Right-click: erase");
            ui.small("Drag: pan");
            ui.small("Scroll: zoom");
            ui.small("Q / E: rotate CCW / CW");
        });
    }

    fn draw_hex(&self, painter: &egui::Painter, hex: Hex, sc: egui::Pos2) {
        let corners = hex_corners(sc, self.zoom);
        if let Some(tile) = self.tiling.tiles.get(&hex) {
            let (type_idx, rotation) = tile_id(tile).unwrap_or((0, 0));
            painter.add(egui::Shape::convex_polygon(
                corners,
                TILE_COLORS[type_idx],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(25, 25, 25)),
            ));
            if self.show_names && self.zoom > 25.0 {
                let galley = painter.layout_no_wrap(
                    TILE_NAMES[type_idx].to_string(),
                    egui::FontId::proportional(self.zoom * 0.32),
                    egui::Color32::WHITE,
                );
                let sz = galley.size();
                // CCW visual rotation: negative angle in egui's CW-positive convention.
                // Each brush.rotation step = 60° CCW.
                let angle = -(rotation as f32) * std::f32::consts::PI / 3.0;
                let (cos_a, sin_a) = (angle.cos(), angle.sin());
                // Place pos (top-left pivot) so the text center lands at sc after rotation.
                // Rotation matrix (CW-positive, y-down): x'=x*cos-y*sin, y'=x*sin+y*cos
                let pos = egui::pos2(
                    sc.x - (sz.x / 2.0 * cos_a - sz.y / 2.0 * sin_a),
                    sc.y - (sz.x / 2.0 * sin_a + sz.y / 2.0 * cos_a),
                );
                let mut text_shape =
                    egui::epaint::TextShape::new(pos, galley, egui::Color32::WHITE);
                text_shape.angle = angle;
                painter.add(egui::Shape::Text(text_shape));
            }
            if self.show_paths && self.zoom > 35.0 {
                if let Some(tree) = &self.tree {
                    if let Some(c) = tree.coords.get(&hex) {
                        painter.text(
                            sc + egui::vec2(0.0, self.zoom * 0.45),
                            egui::Align2::CENTER_CENTER,
                            coords_str(tree.top, c),
                            egui::FontId::monospace(self.zoom * 0.16),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 210),
                        );
                    }
                }
            }
        } else {
            painter.add(egui::Shape::closed_line(
                corners,
                egui::Stroke::new(
                    0.5,
                    egui::Color32::from_rgba_unmultiplied(180, 180, 180, 60),
                ),
            ));
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
            let inset = egui::pos2(
                mid.x + (sc.x - mid.x) * 0.25,
                mid.y + (sc.y - mid.y) * 0.25,
            );
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
    // rim of the patch).  Thickness grows with the square root of the order;
    // higher orders are drawn last so they paint over lower ones.
    fn draw_tree_borders(
        &self,
        tree: &TreeState,
        painter: &egui::Painter,
        hexes: &[Hex],
        canvas_center: egui::Pos2,
    ) {
        let t = Transducer::global();
        let mut segments: Vec<(usize, [egui::Pos2; 2])> = Vec::new();
        for &hex in hexes {
            let Some(c) = tree.coords.get(&hex) else { continue };
            let Some((_, rho)) = self.tiling.tiles.get(&hex).and_then(tile_id) else {
                continue;
            };
            let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
            for (w, &dir) in DIRECTIONS.iter().enumerate() {
                // Each interior edge once (from its E/NE/NW side); rim edges
                // are always drawn from the tile side.
                if w >= 3 && tree.coords.contains_key(&(hex + dir)) {
                    continue;
                }
                let delta = ((w + 6 - rho) % 6) as u8;
                let order = t.border_order(tree.top, c, delta);
                if order > 0 {
                    segments.push((order, edge_endpoints(sc, self.zoom, w)));
                }
            }
        }
        segments.sort_unstable_by_key(|&(order, _)| order);
        for (order, seg) in segments {
            let width = ((self.zoom * 0.045).max(1.5) * (order as f32).sqrt())
                .min(self.zoom * 0.4);
            painter.line_segment(seg, egui::Stroke::new(width, egui::Color32::WHITE));
        }
    }

    fn draw_supertile_outlines(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        canvas_center: egui::Pos2,
    ) {
        let stroke = egui::Stroke::new(
            (self.zoom * 0.08).max(2.0),
            egui::Color32::WHITE,
        );
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
                let mut text_shape =
                    egui::epaint::TextShape::new(pos, galley, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200));
                text_shape.angle = angle;
                painter.add(egui::Shape::Text(text_shape));
            }
            if zoom > 35.0 {
                let font_size = (zoom * 0.20).max(8.0);
                for i in 0..6 {
                    let [a, b] = edge_endpoints(sc, zoom, i);
                    let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                    let inset = egui::pos2(
                        mid.x + (sc.x - mid.x) * 0.25,
                        mid.y + (sc.y - mid.y) * 0.25,
                    );
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

        // Place on left click
        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let hex = screen_to_hex(pos, self.zoom, self.pan, canvas_center);
                let was_empty = self.tiling.tiles.is_empty();
                let patch = self.placement_patch();
                for (&h, tile) in &patch.tiles {
                    self.tiling.insert(h + hex, tile.clone());
                }
                self.invalid_edges = recompute_invalid(&self.tiling);
                // A patch on an empty canvas roots a fresh hierarchy; any
                // other placement is free-form and orphans the coords.
                self.tree = was_empty.then(|| self.placed_tree_state(hex));
            }
        }

        // Erase single tile on right click
        if response.secondary_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let hex = screen_to_hex(pos, self.zoom, self.pan, canvas_center);
                if self.tiling.tiles.remove(&hex).is_some() {
                    self.tree = None;
                }
                self.invalid_edges = recompute_invalid(&self.tiling);
            }
        }
    }
}

impl eframe::App for HexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.side_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = response.rect;
            let canvas_center = rect.center();

            self.handle_input(ctx, &response, canvas_center);

            let hexes = visible_hexes(rect, self.zoom, self.pan);

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
                if let Some(tree) = &self.tree {
                    self.draw_tree_borders(tree, &painter, &hexes, canvas_center);
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
        });
    }
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
                let mut app = HexApp {
                    mode: PlaceMode::Supertile,
                    brush: Brush { type_idx: top, rotation: rot },
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
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Spectre Hex Tile Editor",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 800.0])
                .with_title("Spectre Hex Tile Editor"),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(HexApp::default()))),
    )
}
