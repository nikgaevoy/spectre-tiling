use spectre_tiling::marked::MarkedTiling;
use spectre_tiling::spectre::Label;
use spectre_tiling::supertile::{supertile_gamma, supertile_delta, supertile_theta};
use spectre_tiling::tiling::tile_id;
use std::fmt::Write as FmtWrite;
use std::f64::consts::PI;
use std::fs;

const FILL: [&str; 9] = [
    "#c87828", // Γ  orange-brown
    "#8b3a10", // Δ  dark brown
    "#4a9090", // Θ  teal
    "#3a8050", // Λ  green
    "#c8a848", // Ξ  tan
    "#c04060", // Π  crimson
    "#4878a0", // Σ  steel-blue
    "#a8c828", // Φ  lime
    "#789848", // Ψ  olive-green
];

const NAMES: [&str; 9] = ["Γ", "Δ", "Θ", "Λ", "Ξ", "Π", "Σ", "Φ", "Ψ"];

fn hex_to_pixel(q: i32, r: i32, size: f64) -> (f64, f64) {
    let x = size * (3f64.sqrt() * q as f64 + 3f64.sqrt() / 2.0 * r as f64);
    let y = size * (-1.5 * r as f64);
    (x, y)
}

fn corners(cx: f64, cy: f64, size: f64) -> [(f64, f64); 6] {
    std::array::from_fn(|i| {
        let a = PI / 180.0 * (30.0 + 60.0 * i as f64);
        (cx + size * a.cos(), cy + size * a.sin())
    })
}

fn render(tiling: &MarkedTiling<Label>, title: &str) -> String {
    let s = 60.0_f64;
    let pad = 20.0_f64;

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for &hex in tiling.tiles.keys() {
        let (cx, cy) = hex_to_pixel(hex.q, hex.r, s);
        min_x = min_x.min(cx - s);
        min_y = min_y.min(cy - s);
        max_x = max_x.max(cx + s);
        max_y = max_y.max(cy + s);
    }

    let title_h = 30.0;
    let ox = -min_x + pad;
    let oy = -min_y + pad + title_h;
    let width = max_x - min_x + 2.0 * pad;
    let height = max_y - min_y + 2.0 * pad + title_h;

    let mut svg = String::new();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}">"#,
        width, height
    ).unwrap();
    writeln!(svg, r##"<rect width="{:.0}" height="{:.0}" fill="#f8f4ec"/>"##, width, height).unwrap();
    writeln!(
        svg,
        r##"<text x="{:.1}" y="{:.1}" font-family="sans-serif" font-size="18" text-anchor="middle" fill="#333">Supertile {title}</text>"##,
        width / 2.0, title_h * 0.7
    ).unwrap();
    writeln!(
        svg,
        r#"<style>text.lbl{{font-family:sans-serif;font-size:{:.1}px;text-anchor:middle;dominant-baseline:central;fill:#fff;font-weight:bold}}</style>"#,
        s * 0.38
    ).unwrap();

    let mut entries: Vec<_> = tiling.tiles.iter().collect();
    entries.sort_by_key(|&(&h, _)| (h.r, h.q));

    for &(&hex, tile) in &entries {
        let (cx, cy) = hex_to_pixel(hex.q, hex.r, s);
        let (cx, cy) = (cx + ox, cy + oy);

        let (fill, label, rot) = match tile_id(tile) {
            Some((ti, rot)) => (FILL[ti], NAMES[ti], rot),
            None => ("#aaaaaa", "?", 0),
        };

        let pts: String = corners(cx, cy, s - 1.5)
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ");

        let angle = -(rot as f64) * 60.0;

        writeln!(
            svg,
            r##"<polygon points="{pts}" fill="{fill}" stroke="#1a1a1a" stroke-width="1.2"/>"##
        ).unwrap();
        writeln!(
            svg,
            r#"<text class="lbl" x="{cx:.1}" y="{cy:.1}" transform="rotate({angle:.0},{cx:.1},{cy:.1})">{label}</text>"#
        ).unwrap();
    }

    svg.push_str("</svg>\n");
    svg
}

fn main() {
    let supertiles = [
        ("Γ", supertile_gamma(), "supertile_gamma.svg"),
        ("Δ", supertile_delta(), "supertile_delta.svg"),
        ("Θ", supertile_theta(), "supertile_theta.svg"),
    ];

    for (name, tiling, filename) in &supertiles {
        let svg = render(tiling, name);
        fs::write(filename, &svg).unwrap_or_else(|e| panic!("failed to write {filename}: {e}"));
        println!("Wrote {filename}");
    }
}
