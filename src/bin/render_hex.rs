use spectre_tiling::hex::Hex;
use spectre_tiling::svg::{DrawConfig, draw};
use std::fs;

fn main() {
    let hexes = Hex::spiral(Hex::new(0, 0), 9);
    let svg = draw(&hexes, &DrawConfig::default());
    fs::write("hex_tiling.svg", &svg).expect("failed to write SVG");
    println!("Wrote {} hexes to hex_tiling.svg", hexes.len());
}
