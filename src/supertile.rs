use crate::hex::Hex;
use crate::marked::MarkedTiling;
use crate::spectre::*;

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