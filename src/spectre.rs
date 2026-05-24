use std::ops::Neg;

use crate::marked::MarkedTile;

/// Edge-label alphabet for the 9 spectre marked tiles: ±α, ±β, ±γ, ±δ, ±ε, ±ζ, η, ±θ.
///
/// Encoding (i8 discriminants):
///   α=1  β=2  γ=3  δ=4  ε=5  ζ=6  θ=7  η=0
///
/// η carries no sign because −η = η; this is automatic since −0 = 0.
/// Negation for all other labels is ordinary i8 negation.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    NegTheta = -7,
    NegZeta = -6,
    NegEpsilon = -5,
    NegDelta = -4,
    NegGamma = -3,
    NegBeta = -2,
    NegAlpha = -1,
    Eta = 0,
    Alpha = 1,
    Beta = 2,
    Gamma = 3,
    Delta = 4,
    Epsilon = 5,
    Zeta = 6,
    Theta = 7,
}

impl Neg for Label {
    type Output = Self;
    fn neg(self) -> Self {
        // SAFETY: every integer in −7..=7 is a valid discriminant, and negating
        // any value in that range stays within it, so the transmute is sound.
        unsafe { std::mem::transmute(-(self as i8)) }
    }
}

// TODO: fill in edge labels for all tiles below
pub const GAMMA: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const DELTA: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const THETA: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const LAMBDA: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const XI: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const PI: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const SIGMA: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const PHI: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
pub const PSI: MarkedTile<Label> = MarkedTile::new([Label::Eta; 6]);
