use std::collections::HashSet;

use eframe::egui;
use spectre_tiling::hex::{Hex, DIRECTIONS};
use spectre_tiling::marked::{MarkedTile, MarkedTiling};
use spectre_tiling::spectre::Label;
use spectre_tiling::supertile::{
    supertile_delta, supertile_gamma, supertile_lambda, supertile_phi, supertile_pi,
    supertile_psi, supertile_sigma, supertile_theta, supertile_xi,
    AnchorPoint, SUPERTILE_ANCHORS,
};
use spectre_tiling::tiling::{tile_id, BASE_TILES, TILE_NAMES};

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
    AnchorEdit,
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
    // (type_idx, rotation, origin) for each placed supertile, used to render anchors.
    placed_supertiles: Vec<(usize, usize, Hex)>,
    editable_anchors: [[AnchorPoint; 6]; 9],
    active_anchor_slot: usize,
    hover_corner: Option<(Hex, u8)>,
    anchor_copy_feedback: f32,
    save_feedback_timer: f32,
    save_error: String,
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
            placed_supertiles: Vec::new(),
            editable_anchors: SUPERTILE_ANCHORS,
            active_anchor_slot: 0,
            hover_corner: None,
            anchor_copy_feedback: 0.0,
            save_feedback_timer: 0.0,
            save_error: String::new(),
        }
    }
}

fn find_nearest_corner(
    supertile: &MarkedTiling<Label>,
    cursor: egui::Pos2,
    zoom: f32,
    pan: egui::Vec2,
    canvas_center: egui::Pos2,
) -> Option<(Hex, u8)> {
    let threshold = (zoom * 0.35).max(10.0);
    let mut best: Option<(f32, Hex, u8)> = None;
    for (&hex, _) in &supertile.tiles {
        let sc = hex_to_screen(hex, zoom, pan, canvas_center);
        for c in 0u8..6 {
            let pos = corner(sc, zoom, c as usize);
            let dist = (pos - cursor).length();
            if dist < threshold {
                if best.map_or(true, |(d, _, _)| dist < d) {
                    best = Some((dist, hex, c));
                }
            }
        }
    }
    best.map(|(_, h, c)| (h, c))
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
            PlaceMode::AnchorEdit => MarkedTiling::new(),
        }
    }

    fn generate_anchor_code(&self) -> String {
        let anchors = &self.editable_anchors[self.brush.type_idx];
        let aps: Vec<String> = anchors
            .iter()
            .map(|ap| format!("ap({},{},{})", ap.hex.q, ap.hex.r, ap.corner))
            .collect();
        format!("[{}]", aps.join(", "))
    }

    fn generate_all_anchors_code(&self) -> String {
        let names = [
            "Γ (supertile_gamma)", "Δ (supertile_delta)", "Θ (supertile_theta)",
            "Λ (supertile_lambda)", "Ξ (supertile_xi)", "Π (supertile_pi)",
            "Σ (supertile_sigma)", "Φ (supertile_phi)", "Ψ (supertile_psi)",
        ];
        let mut lines = Vec::new();
        for (i, anchors) in self.editable_anchors.iter().enumerate() {
            let aps: Vec<String> = anchors
                .iter()
                .map(|ap| format!("ap({},{},{})", ap.hex.q, ap.hex.r, ap.corner))
                .collect();
            lines.push(format!("    // {}", names[i]));
            lines.push(format!("    [{}],", aps.join(", ")));
        }
        lines.join("\n")
    }

    fn save_anchors_to_source(&self) -> Result<(), String> {
        let path = "src/supertile.rs";
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Read error: {e}"))?;

        let marker = "pub const SUPERTILE_ANCHORS: [[AnchorPoint; 6]; 9] = [";
        let start = content
            .find(marker)
            .ok_or_else(|| "SUPERTILE_ANCHORS not found in supertile.rs".to_string())?;
        let after_bracket = start + marker.len();

        let end_rel = content[after_bracket..]
            .find("\n];")
            .ok_or_else(|| "Closing ]; not found in SUPERTILE_ANCHORS".to_string())?;

        let prefix = &content[..after_bracket];
        let suffix = &content[after_bracket + end_rel..]; // starts with "\n];"
        let inner = self.generate_all_anchors_code();
        let new_content = format!("{}\n{}{}", prefix, inner, suffix);

        std::fs::write(path, new_content).map_err(|e| format!("Write error: {e}"))
    }

    fn side_panel(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt);
        self.anchor_copy_feedback = (self.anchor_copy_feedback - dt).max(0.0);
        self.save_feedback_timer = (self.save_feedback_timer - dt).max(0.0);
        egui::SidePanel::left("tools").min_width(130.0).show(ctx, |ui| {
            // Mode toggle
            ui.heading("Mode");
            ui.separator();
            for &(label, mode) in &[
                ("Single", PlaceMode::Single),
                ("Supertile", PlaceMode::Supertile),
                ("Anchor Edit", PlaceMode::AnchorEdit),
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

            if self.mode != PlaceMode::AnchorEdit {
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
            } else {
                // Anchor slot editor
                ui.add_space(10.0);
                ui.heading("Anchor Slots");
                ui.separator();
                let slot_colors = [
                    egui::Color32::from_rgb(255, 80, 80),
                    egui::Color32::from_rgb(80, 200, 80),
                    egui::Color32::from_rgb(80, 120, 255),
                    egui::Color32::from_rgb(220, 180, 0),
                    egui::Color32::from_rgb(200, 80, 200),
                    egui::Color32::from_rgb(0, 190, 190),
                ];
                for slot in 0..6 {
                    let ap = self.editable_anchors[self.brush.type_idx][slot];
                    let is_active = self.active_anchor_slot == slot;
                    let color = slot_colors[slot];
                    let text = format!("{}: ({},{}) c{}", slot, ap.hex.q, ap.hex.r, ap.corner);
                    let fill = if is_active {
                        color
                    } else {
                        egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60)
                    };
                    let text_color = if is_active {
                        egui::Color32::BLACK
                    } else {
                        color
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(text).color(text_color).strong(),
                            )
                            .fill(fill)
                            .min_size(egui::vec2(115.0, 22.0)),
                        )
                        .clicked()
                    {
                        self.active_anchor_slot = slot;
                    }
                }
                ui.add_space(6.0);
                ui.small("Click canvas corner to set slot.");
                ui.add_space(4.0);
                if ui.button("Copy Current").clicked() {
                    let code = self.generate_anchor_code();
                    ctx.copy_text(code);
                    self.anchor_copy_feedback = 2.0;
                }
                if ui.button("Copy All").clicked() {
                    let code = self.generate_all_anchors_code();
                    ctx.copy_text(code);
                    self.anchor_copy_feedback = 2.0;
                }
                if self.anchor_copy_feedback > 0.0 {
                    ui.colored_label(egui::Color32::from_rgb(50, 200, 80), "Copied!");
                    ctx.request_repaint();
                }
                ui.add_space(4.0);
                if ui.button("Save to supertile.rs").clicked() {
                    match self.save_anchors_to_source() {
                        Ok(()) => {
                            self.save_error = String::new();
                            self.save_feedback_timer = 3.0;
                        }
                        Err(e) => {
                            self.save_error = e;
                            self.save_feedback_timer = 5.0;
                        }
                    }
                }
                if self.save_feedback_timer > 0.0 {
                    if self.save_error.is_empty() {
                        ui.colored_label(
                            egui::Color32::from_rgb(50, 200, 80),
                            "Saved to supertile.rs ✓",
                        );
                    } else {
                        ui.colored_label(
                            egui::Color32::from_rgb(220, 60, 40),
                            &self.save_error.clone(),
                        );
                    }
                    ctx.request_repaint();
                }
            }

            ui.add_space(10.0);
            ui.separator();
            if ui.button("Clear All").clicked() {
                self.tiling = MarkedTiling::new();
                self.invalid_edges.clear();
                self.placed_supertiles.clear();
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
            ui.separator();
            ui.small("Left-click: place");
            ui.small("Right-click: erase");
            ui.small("Drag: pan");
            ui.small("Scroll: zoom");
        });
    }

    fn draw_hex(&self, painter: &egui::Painter, hex: Hex, sc: egui::Pos2) {
        let corners = hex_corners(sc, self.zoom);
        let tile_in_tiling = if self.mode != PlaceMode::AnchorEdit {
            self.tiling.tiles.get(&hex)
        } else {
            None
        };
        if let Some(tile) = tile_in_tiling {
            let (type_idx, rotation) = tile_id(tile).unwrap_or((0, 0));
            painter.add(egui::Shape::convex_polygon(
                corners,
                TILE_COLORS[type_idx],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(25, 25, 25)),
            ));
            if self.zoom > 25.0 {
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

    fn draw_anchor_points(
        painter: &egui::Painter,
        anchors: &[AnchorPoint; 6],
        offset: Hex,
        zoom: f32,
        pan: egui::Vec2,
        canvas_center: egui::Pos2,
        alpha: u8,
    ) {
        let radius = (zoom * 0.12).max(3.0);
        for ap in anchors {
            let sc = hex_to_screen(ap.hex + offset, zoom, pan, canvas_center);
            let pos = corner(sc, zoom, ap.corner as usize);
            painter.circle(
                pos,
                radius,
                egui::Color32::from_rgba_unmultiplied(255, 220, 40, alpha),
                egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha)),
            );
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

        // Track nearest hex corner in anchor edit mode
        if self.mode == PlaceMode::AnchorEdit {
            let supertile = BASE_SUPERTILE_FNS[self.brush.type_idx]();
            self.hover_corner = response.hover_pos().and_then(|cursor| {
                find_nearest_corner(&supertile, cursor, self.zoom, self.pan, canvas_center)
            });
        } else {
            self.hover_corner = None;
        }

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
            if self.mode == PlaceMode::AnchorEdit {
                if let Some((hex, c)) = self.hover_corner {
                    self.editable_anchors[self.brush.type_idx][self.active_anchor_slot] =
                        AnchorPoint::new(hex, c);
                    self.active_anchor_slot = (self.active_anchor_slot + 1) % 6;
                }
            } else if let Some(pos) = response.interact_pointer_pos() {
                let hex = screen_to_hex(pos, self.zoom, self.pan, canvas_center);
                let patch = self.placement_patch();
                for (&h, tile) in &patch.tiles {
                    self.tiling.insert(h + hex, tile.clone());
                }
                self.invalid_edges = recompute_invalid(&self.tiling);
                if self.mode == PlaceMode::Supertile {
                    self.placed_supertiles.push((self.brush.type_idx, self.brush.rotation, hex));
                }
            }
        }

        // Erase single tile on right click (not in anchor edit mode)
        if response.secondary_clicked() && self.mode != PlaceMode::AnchorEdit {
            if let Some(pos) = response.interact_pointer_pos() {
                let hex = screen_to_hex(pos, self.zoom, self.pan, canvas_center);
                self.tiling.tiles.remove(&hex);
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

            if self.mode != PlaceMode::AnchorEdit {
                for &hex in &hexes {
                    let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                    self.draw_invalid_edges(&painter, hex, sc);
                }
            }

            if self.zoom > 35.0 && self.mode != PlaceMode::AnchorEdit {
                for &hex in &hexes {
                    if let Some(tile) = self.tiling.tiles.get(&hex).cloned() {
                        let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                        Self::draw_edge_labels(&painter, &tile, sc, self.zoom);
                    }
                }
            }

            // Anchor points for placed supertiles
            if self.mode == PlaceMode::Supertile {
                for &(type_idx, rotation, origin) in &self.placed_supertiles {
                    let anchors = SUPERTILE_ANCHORS[type_idx].map(|ap| ap.rotate(rotation));
                    Self::draw_anchor_points(&painter, &anchors, origin, self.zoom, self.pan, canvas_center, 255);
                }
            }

            // Ghost preview of pending placement at hover position
            if let Some(hover) = self.hover_hex {
                if matches!(self.mode, PlaceMode::Single | PlaceMode::Supertile) {
                    let patch = self.placement_patch();
                    Self::draw_ghost(&painter, &patch, hover, self.zoom, self.pan, canvas_center);
                    if self.mode == PlaceMode::Supertile {
                        let anchors = SUPERTILE_ANCHORS[self.brush.type_idx].map(|ap| ap.rotate(self.brush.rotation));
                        Self::draw_anchor_points(&painter, &anchors, hover, self.zoom, self.pan, canvas_center, 180);
                    }
                }
            }

            // Anchor edit overlay
            if self.mode == PlaceMode::AnchorEdit {
                let supertile = BASE_SUPERTILE_FNS[self.brush.type_idx]();

                // Draw supertile tiles
                for (&hex, tile) in &supertile.tiles {
                    let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                    let (type_idx, _) = tile_id(tile).unwrap_or((0, 0));
                    painter.add(egui::Shape::convex_polygon(
                        hex_corners(sc, self.zoom),
                        TILE_COLORS[type_idx],
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(25, 25, 25)),
                    ));
                }

                // Draw all hex corners as small dots
                let dot_r = (self.zoom * 0.08).max(2.5);
                for (&hex, _) in &supertile.tiles {
                    let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                    for c in 0u8..6 {
                        let pos = corner(sc, self.zoom, c as usize);
                        painter.circle(
                            pos,
                            dot_r,
                            egui::Color32::from_rgba_unmultiplied(220, 220, 220, 180),
                            egui::Stroke::NONE,
                        );
                    }
                }

                // Draw assigned anchor slots
                let slot_colors = [
                    egui::Color32::from_rgb(255, 80, 80),
                    egui::Color32::from_rgb(80, 200, 80),
                    egui::Color32::from_rgb(80, 120, 255),
                    egui::Color32::from_rgb(220, 180, 0),
                    egui::Color32::from_rgb(200, 80, 200),
                    egui::Color32::from_rgb(0, 190, 190),
                ];
                for slot in 0..6 {
                    let ap = self.editable_anchors[self.brush.type_idx][slot];
                    let sc = hex_to_screen(ap.hex, self.zoom, self.pan, canvas_center);
                    let pos = corner(sc, self.zoom, ap.corner as usize);
                    let is_active = slot == self.active_anchor_slot;
                    let r = if is_active {
                        (self.zoom * 0.18).max(6.0)
                    } else {
                        (self.zoom * 0.13).max(4.0)
                    };
                    painter.circle(
                        pos,
                        r,
                        slot_colors[slot],
                        egui::Stroke::new(2.0, egui::Color32::BLACK),
                    );
                    painter.text(
                        pos,
                        egui::Align2::CENTER_CENTER,
                        slot.to_string(),
                        egui::FontId::proportional((self.zoom * 0.14).max(7.0)),
                        egui::Color32::BLACK,
                    );
                }

                // Hover corner highlight
                if let Some((hh, hc)) = self.hover_corner {
                    let sc = hex_to_screen(hh, self.zoom, self.pan, canvas_center);
                    let pos = corner(sc, self.zoom, hc as usize);
                    painter.circle(
                        pos,
                        (self.zoom * 0.22).max(7.0),
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                        egui::Stroke::new(2.5, egui::Color32::from_rgb(50, 200, 255)),
                    );
                }
            }
        });
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
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "NotoSans".into(),
                egui::FontData::from_static(include_bytes!(
                    "../../assets/NotoSans.ttf"
                )).into(),
            );
            fonts.font_data.insert(
                "NotoSansSymbols".into(),
                egui::FontData::from_static(include_bytes!(
                    "../../assets/NotoSansSymbols.ttf"
                )).into(),
            );
            let proportional = fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default();
            proportional.insert(0, "NotoSans".into());
            proportional.push("NotoSansSymbols".into());
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(HexApp::default()))
        }),
    )
}
