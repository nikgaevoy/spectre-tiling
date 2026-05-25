use std::collections::HashSet;

use eframe::egui;
use spectre_tiling::hex::{Hex, DIRECTIONS};
use spectre_tiling::marked::{MarkedTile, MarkedTiling};
use spectre_tiling::spectre::Label;
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

struct Brush {
    type_idx: usize,
    rotation: usize,
}

struct HexApp {
    tiling: MarkedTiling<Label>,
    invalid_edges: HashSet<(Hex, usize)>,
    brush: Brush,
    pan: egui::Vec2,
    zoom: f32,
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
            pan: egui::Vec2::ZERO,
            zoom: 50.0,
        }
    }
}

impl HexApp {
    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tools").min_width(130.0).show(ctx, |ui| {
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

            // Tile preview
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

            ui.add_space(10.0);
            ui.separator();
            if ui.button("Clear All").clicked() {
                self.tiling = MarkedTiling::new();
                self.invalid_edges.clear();
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
        if let Some(tile) = self.tiling.tiles.get(&hex) {
            let type_idx = tile_id(tile).map(|(ti, _)| ti).unwrap_or(0);
            painter.add(egui::Shape::convex_polygon(
                corners,
                TILE_COLORS[type_idx],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(25, 25, 25)),
            ));
            if self.zoom > 25.0 {
                painter.text(
                    sc,
                    egui::Align2::CENTER_CENTER,
                    TILE_NAMES[type_idx],
                    egui::FontId::proportional(self.zoom * 0.32),
                    egui::Color32::WHITE,
                );
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

    fn draw_edge_labels(painter: &egui::Painter, tile: &MarkedTile<Label>, sc: egui::Pos2, zoom: f32) {
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

    fn handle_input(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        canvas_center: egui::Pos2,
    ) {
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

        // Place tile on left click (egui only fires clicked() when movement was small)
        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let hex = screen_to_hex(pos, self.zoom, self.pan, canvas_center);
                let tile = BASE_TILES[self.brush.type_idx].rotate(self.brush.rotation);
                self.tiling.insert(hex, tile);
                self.invalid_edges = recompute_invalid(&self.tiling);
            }
        }

        // Erase on right click
        if response.secondary_clicked() {
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

            for &hex in &hexes {
                let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                self.draw_invalid_edges(&painter, hex, sc);
            }

            if self.zoom > 35.0 {
                for &hex in &hexes {
                    if let Some(tile) = self.tiling.tiles.get(&hex).cloned() {
                        let sc = hex_to_screen(hex, self.zoom, self.pan, canvas_center);
                        Self::draw_edge_labels(&painter, &tile, sc, self.zoom);
                    }
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
        Box::new(|_cc| Ok(Box::new(HexApp::default()))),
    )
}
