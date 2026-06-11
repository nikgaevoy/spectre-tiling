use crate::hex::Hex;
use crate::marked::MarkedTiling;
use crate::spectre::*;

/// A specific vertex on a supertile border: the `corner`-th vertex (0–5) of `hex`.
/// Both fields are in the supertile's local coordinate frame (origin = Hex(0,0)).
/// Corner numbering matches `spectre_explorer::corner()`: angle = 30° + 60°·i, y-down screen space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorPoint {
    pub hex: Hex,
    pub corner: u8,
}

impl AnchorPoint {
    pub const fn new(hex: Hex, corner: u8) -> Self {
        Self { hex, corner }
    }

    /// Rotate by `n` 60° steps, consistent with `rotate_tiling`.
    /// Each step: hex → hex.rotate_cw(), corner → (corner + 5) % 6.
    pub fn rotate(self, n: usize) -> Self {
        let mut h = self.hex;
        for _ in 0..n {
            h = h.rotate_cw();
        }
        Self {
            hex: h,
            corner: ((self.corner as usize + 6 - n % 6) % 6) as u8,
        }
    }

    /// Convert from supertile-local to global hex coordinates.
    pub fn translate(self, origin: Hex) -> Self {
        Self { hex: self.hex + origin, ..self }
    }
}

const fn ap(q: i32, r: i32, corner: u8) -> AnchorPoint {
    AnchorPoint::new(Hex::new(q, r), corner)
}

/// Base anchor points (rotation 0) for each supertile type.
/// Index order: Γ=0 Δ=1 Θ=2 Λ=3 Ξ=4 Π=5 Σ=6 Φ=7 Ψ=8  (matches BASE_TILES / TILE_NAMES).
/// Fill in hex coordinates and corner indices (0–5) manually.
pub const SUPERTILE_ANCHORS: [[AnchorPoint; 6]; 9] = [
    // Γ (supertile_gamma)
    [ap(0,-1,3), ap(0,0,3), ap(0,1,4), ap(1,1,5), ap(2,-1,0), ap(0,-1,1)],
    // Δ (supertile_delta)
    [ap(0,-1,2), ap(0,0,3), ap(1,1,4), ap(1,1,0), ap(2,-1,0), ap(2,-2,1)],
    // Θ (supertile_theta)
    [ap(0,-1,3), ap(0,0,3), ap(1,1,4), ap(1,1,0), ap(2,-2,0), ap(2,-2,2)],
    // Λ (supertile_lambda)
    [ap(0,-1,3), ap(0,0,3), ap(1,1,4), ap(1,1,0), ap(2,-1,0), ap(2,-2,1)],
    // Ξ (supertile_xi)
    [ap(0,-1,3), ap(0,0,3), ap(0,1,4), ap(1,1,0), ap(2,-2,0), ap(2,-2,2)],
    // Π (supertile_pi)
    [ap(0,-1,3), ap(0,0,3), ap(0,1,4), ap(1,1,0), ap(2,-1,0), ap(2,-2,1)],
    // Σ (supertile_sigma)
    [ap(0,-1,2), ap(0,1,3), ap(1,1,4), ap(1,1,0), ap(2,-1,0), ap(2,-2,1)],
    // Φ (supertile_phi)
    [ap(0,-1,3), ap(0,0,3), ap(1,1,4), ap(1,1,0), ap(2,-1,0), ap(2,-2,2)],
    // Ψ (supertile_psi)
    [ap(0,-1,3), ap(0,0,3), ap(0,1,4), ap(1,1,0), ap(2,-1,0), ap(2,-2,2)],
];

pub const BASE_SUPERTILE_FNS: [fn() -> MarkedTiling<Label>; 9] = [
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

pub fn supertile_gamma() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, -1), PI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), THETA);
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), XI.rotate(1));

    t
}

pub fn supertile_theta() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), PI.rotate(1));
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, -1), PSI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(2, -2), PI.rotate(5));

    t
}

pub fn supertile_lambda() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), PI.rotate(1));
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, -1), PSI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(2, -2), XI.rotate(5));

    t
}

pub fn supertile_xi() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), PSI.rotate(1));
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, -1), PSI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(2, -2), PI.rotate(5));

    t
}

pub fn supertile_pi() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), PSI.rotate(1));
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, -1), PSI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(2, -2), XI.rotate(5));

    t
}

pub fn supertile_sigma() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, 1), LAMBDA.rotate(2));
    t.insert(Hex::new(1, 1), PI.rotate(1));
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, -1), XI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(2, -2), XI.rotate(5));

    t
}

pub fn supertile_phi() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), PI.rotate(1));
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, -1), PSI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(2, -2), PSI.rotate(5));

    t
}

pub fn supertile_psi() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), PSI.rotate(1));
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, -1), PSI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(2, -2), PSI.rotate(5));

    t
}

pub fn supertile_delta() -> MarkedTiling<Label> {
    let mut t = MarkedTiling::new();

    t.insert(Hex::new(2, -2), XI.rotate(5));
    t.insert(Hex::new(0, -1), XI.rotate(4));
    t.insert(Hex::new(1, -1), DELTA.rotate(5));
    t.insert(Hex::new(2, -1), PHI);
    t.insert(Hex::new(0, 0), GAMMA);
    t.insert(Hex::new(1, 0), SIGMA.rotate(1));
    t.insert(Hex::new(0, 1), PHI.rotate(2));
    t.insert(Hex::new(1, 1), PI.rotate(1));

    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supertiles_valid() {
        assert!(supertile_gamma().is_valid());
        assert!(supertile_delta().is_valid());
        assert!(supertile_theta().is_valid());
        assert!(supertile_lambda().is_valid());
        assert!(supertile_xi().is_valid());
        assert!(supertile_pi().is_valid());
        assert!(supertile_sigma().is_valid());
        assert!(supertile_phi().is_valid());
        assert!(supertile_psi().is_valid());
    }
}