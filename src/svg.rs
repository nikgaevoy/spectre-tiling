use crate::hex::Hex;
use std::f64::consts::PI;
use std::fmt::Write;

/// Configuration for rendering a hex grid to SVG.
pub struct DrawConfig {
    /// Circumradius of each hexagon (center to vertex).
    pub hex_size: f64,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    /// Label font size; `None` auto-sizes to 30 % of `hex_size`.
    pub font_size: Option<f64>,
    /// Padding around the grid in pixels.
    pub padding: f64,
}

impl Default for DrawConfig {
    fn default() -> Self {
        DrawConfig {
            hex_size: 50.0,
            fill: "#f0f4ff".to_string(),
            stroke: "#445566".to_string(),
            stroke_width: 1.5,
            font_size: None,
            padding: 10.0,
        }
    }
}

/// Render a collection of hex tiles as an SVG string.
///
/// Uses a **pointy-top** orientation. Each tile is labelled with its
/// axial `(q, r)` coordinates.
pub fn draw(hexes: &[Hex], config: &DrawConfig) -> String {
    if hexes.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0"/>"#.to_string();
    }

    let s = config.hex_size;
    let font_size = config.font_size.unwrap_or(s * 0.30);
    let pad = config.padding;

    // Compute bounding box in pixel space (without offset).
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for &h in hexes {
        let (cx, cy) = hex_to_pixel(h, s);
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
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.1}" height="{height:.1}" viewBox="0 0 {width:.1} {height:.1}">"#,
    )
    .unwrap();
    write!(
        svg,
        r#"<style>polygon{{fill:{fill};stroke:{stroke};stroke-width:{sw:.1}}}text{{font-family:monospace;font-size:{fs:.1}px;text-anchor:middle;dominant-baseline:central;fill:{stroke}}}</style>"#,
        fill = config.fill,
        stroke = config.stroke,
        sw = config.stroke_width,
        fs = font_size,
    )
    .unwrap();

    for &h in hexes {
        let (cx, cy) = hex_to_pixel(h, s);
        let (cx, cy) = (cx + ox, cy + oy);

        let pts: String = pointy_top_corners(cx, cy, s)
            .iter()
            .map(|(x, y)| format!("{x:.2},{y:.2}"))
            .collect::<Vec<_>>()
            .join(" ");

        write!(svg, r#"<polygon points="{pts}"/>"#).unwrap();
        write!(
            svg,
            r#"<text x="{cx:.2}" y="{cy:.2}">{},{}</text>"#,
            h.q, h.r
        )
        .unwrap();
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Axial → pixel for a **pointy-top** hex grid.
///
///   x = size * (√3 · q  +  √3/2 · r)
///   y = size * (3/2 · r)
fn hex_to_pixel(hex: Hex, size: f64) -> (f64, f64) {
    let x = size * (3f64.sqrt() * hex.q as f64 + 3f64.sqrt() / 2.0 * hex.r as f64);
    let y = size * (-1.5 * hex.r as f64);
    (x, y)
}

/// Six vertices of a pointy-top hexagon centred at `(cx, cy)`.
/// Corner `i` is at angle `30° + 60°·i` (SVG y-down convention).
fn pointy_top_corners(cx: f64, cy: f64, size: f64) -> [(f64, f64); 6] {
    std::array::from_fn(|i| {
        let angle = PI / 180.0 * (30.0 + 60.0 * i as f64);
        (cx + size * angle.cos(), cy + size * angle.sin())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_empty() {
        let svg = draw(&[], &DrawConfig::default());
        assert!(svg.contains("svg"));
    }

    #[test]
    fn smoke_single() {
        let svg = draw(&[Hex::new(0, 0)], &DrawConfig::default());
        assert!(svg.contains("0,0"));
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn labels_present() {
        let hexes = Hex::spiral(Hex::new(0, 0), 2);
        let svg = draw(&hexes, &DrawConfig::default());
        assert!(svg.contains("2,-1"));
        assert!(svg.contains("-1,2"));
    }

    #[test]
    fn neighbor_pixel_distance() {
        let s = 40.0;
        let (ax, ay) = hex_to_pixel(Hex::new(0, 0), s);
        let (bx, by) = hex_to_pixel(Hex::new(1, 0), s);
        let dist = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
        let expected = 3f64.sqrt() * s;
        assert!((dist - expected).abs() < 1e-9);
    }
}
