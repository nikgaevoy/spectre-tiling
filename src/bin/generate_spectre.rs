use spectre_tiling::tiling::{generate_patch, tile_id};
use spectre_tiling::hex::Hex;
use spectre_tiling::marked::MarkedTiling;
use spectre_tiling::spectre::Label;
use std::fmt::Write as FmtWrite;
use std::f64::consts::PI;
use std::fs;

fn main() {
    let target = 800;
    let tiling = generate_patch(target);

    println!("Placed {} tiles", tiling.tiles.len());
    println!("Valid: {}", tiling.is_valid());

    // Print a brief type breakdown.
    let mut counts = [0usize; 9];
    for tile in tiling.tiles.values() {
        if let Some((ti, _)) = tile_id(tile) {
            counts[ti] += 1;
        }
    }
    let names = ["Γ", "Δ", "Θ", "Λ", "Ξ", "Π", "Σ", "Φ", "Ψ"];
    for (i, &n) in names.iter().enumerate() {
        print!("{}: {}  ", n, counts[i]);
    }
    println!();

    let svg = render_tiling(&tiling);
    fs::write("spectre_tiling.svg", &svg).expect("failed to write SVG");
    println!("Wrote spectre_tiling.svg");
}

// ---------------------------------------------------------------------------
// Colours for the 9 tile types (matching the paper's Figure 4.2 palette)
// ---------------------------------------------------------------------------
const FILL: [&str; 9] = [
    "#c06820", // Γ  orange-brown
    "#7a3010", // Δ  dark brown
    "#207878", // Θ  teal
    "#2d6e40", // Λ  green
    "#b89438", // Ξ  tan
    "#a83050", // Π  crimson
    "#3a6888", // Σ  steel-blue
    "#90b810", // Φ  lime
    "#68a030", // Ψ  medium-green
];

fn render_tiling(tiling: &MarkedTiling<Label>) -> String {
    let s = 28.0_f64;
    let pad = 30.0_f64;

    if tiling.tiles.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#.to_string();
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for &hex in tiling.tiles.keys() {
        let (cx, cy) = hex_to_pixel(hex, s);
        min_x = min_x.min(cx - s);
        min_y = min_y.min(cy - s);
        max_x = max_x.max(cx + s);
        max_y = max_y.max(cy + s);
    }

    let ox = -min_x + pad;
    let oy = -min_y + pad;
    let width = max_x - min_x + 2.0 * pad;
    let height = max_y - min_y + 2.0 * pad;

    let mut svg = String::new();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}">"#,
        width, height
    )
    .unwrap();
    writeln!(svg, r#"<style>text{{font-family:sans-serif;font-size:{:.1}px;text-anchor:middle;dominant-baseline:central;fill:#fff;font-weight:bold}}</style>"#,
        s * 0.38).unwrap();

    // Sort by position for deterministic SVG output.
    let mut entries: Vec<_> = tiling.tiles.iter().collect();
    entries.sort_by_key(|&(&h, _)| (h.r, h.q));

    for &(&hex, tile) in &entries {
        let (cx, cy) = hex_to_pixel(hex, s);
        let (cx, cy) = (cx + ox, cy + oy);

        let (fill, label, rot) = if let Some((ti, rot)) = tile_id(tile) {
            (FILL[ti], ["Γ", "Δ", "Θ", "Λ", "Ξ", "Π", "Σ", "Φ", "Ψ"][ti], rot)
        } else {
            ("#aaaaaa", "?", 0)
        };

        let pts: String = corners(cx, cy, s - 1.0)
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ");

        // Each rotation step is 60°. The hex grid uses CCW rotations; in SVG
        // (y-axis down) CCW corresponds to a negative rotate angle.
        let angle = -(rot as f64) * 60.0;

        writeln!(
            svg,
            r##"<polygon points="{pts}" fill="{fill}" stroke="#1a1a1a" stroke-width="0.8"/>"##
        )
        .unwrap();
        writeln!(
            svg,
            r#"<text x="{cx:.1}" y="{cy:.1}" transform="rotate({angle:.0},{cx:.1},{cy:.1})">{label}</text>"#
        )
        .unwrap();
    }

    svg.push_str("</svg>\n");
    svg
}

fn hex_to_pixel(hex: Hex, size: f64) -> (f64, f64) {
    let x = size * (3f64.sqrt() * hex.q as f64 + 3f64.sqrt() / 2.0 * hex.r as f64);
    let y = size * (-1.5 * hex.r as f64);
    (x, y)
}

fn corners(cx: f64, cy: f64, size: f64) -> [(f64, f64); 6] {
    std::array::from_fn(|i| {
        let a = PI / 180.0 * (30.0 + 60.0 * i as f64);
        (cx + size * a.cos(), cy + size * a.sin())
    })
}
