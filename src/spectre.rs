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

// Edge arrays are indexed [E, NE, NW, W, SW, SE] matching DIRECTIONS[0..6].
use Label::*;
#[rustfmt::skip]
pub const GAMMA:  MarkedTile<Label> = MarkedTile::new([NegDelta, Beta,     NegBeta,  NegAlpha, Alpha,   NegGamma  ]);
#[rustfmt::skip]
pub const DELTA:  MarkedTile<Label> = MarkedTile::new([Alpha,    NegGamma, NegZeta,  Gamma,    Beta,    NegEpsilon]);
#[rustfmt::skip]
pub const THETA:  MarkedTile<Label> = MarkedTile::new([Beta,     Eta,      NegBeta,  Gamma,    Beta,    Theta     ]);
#[rustfmt::skip]
pub const LAMBDA: MarkedTile<Label> = MarkedTile::new([Alpha,    NegTheta, NegBeta,  Gamma,    Beta,    NegEpsilon]);
#[rustfmt::skip]
pub const XI:     MarkedTile<Label> = MarkedTile::new([Beta,     Eta,      NegBeta,  NegAlpha, Epsilon, Theta     ]);
#[rustfmt::skip]
pub const PI:     MarkedTile<Label> = MarkedTile::new([Alpha,    NegTheta, NegBeta,  NegAlpha, Epsilon, NegEpsilon]);
#[rustfmt::skip]
pub const SIGMA:  MarkedTile<Label> = MarkedTile::new([Alpha,    NegGamma, Delta,    Zeta,     Beta,    NegEpsilon]);
#[rustfmt::skip]
pub const PHI:    MarkedTile<Label> = MarkedTile::new([Epsilon,  Eta,      NegBeta,  Gamma,    Beta,    NegEpsilon]);
#[rustfmt::skip]
pub const PSI:    MarkedTile<Label> = MarkedTile::new([Epsilon,  Eta,      NegBeta,  NegAlpha, Epsilon, NegEpsilon]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_six_is_identity() {
        for tile in [GAMMA, DELTA, THETA, LAMBDA, XI, PI, SIGMA, PHI, PSI] {
            assert_eq!(tile.rotate(6).edges, tile.edges);
        }
    }

    #[test]
    fn rotate_ccw_then_cw_is_identity() {
        for tile in [GAMMA, DELTA, THETA, LAMBDA, XI, PI, SIGMA, PHI, PSI] {
            assert_eq!(tile.rotate_ccw().rotate_cw().edges, tile.edges);
            assert_eq!(tile.rotate_cw().rotate_ccw().edges, tile.edges);
        }
    }

    #[test]
    fn rotate_ccw_edge_positions() {
        // After one CCW click, old edge i is now at position (i+1)%6.
        let r = GAMMA.rotate_ccw();
        for i in 0..6 {
            assert_eq!(r.edges[(i + 1) % 6], GAMMA.edges[i]);
        }
    }

    #[test]
    fn rotate_cw_edge_positions() {
        // After one CW click, old edge i is now at position (i+5)%6.
        let r = GAMMA.rotate_cw();
        for i in 0..6 {
            assert_eq!(r.edges[(i + 5) % 6], GAMMA.edges[i]);
        }
    }
}
