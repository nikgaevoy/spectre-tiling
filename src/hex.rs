use std::ops::{Add, Mul, Neg, Sub};

/// Hex cell in axial (q, r) coordinates.
/// The third "cube" coordinate is s = -q - r.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
}

impl Hex {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Third cube coordinate (derived).
    pub fn s(self) -> i32 {
        -self.q - self.r
    }

    pub fn length(self) -> i32 {
        (self.q.abs() + self.r.abs() + self.s().abs()) / 2
    }

    pub fn distance(self, other: Hex) -> i32 {
        (self - other).length()
    }

    pub fn neighbors(self) -> [Hex; 6] {
        DIRECTIONS.map(|d| self + d)
    }

    /// 60° clockwise rotation.
    pub fn rotate_cw(self) -> Self {
        Hex::new(-self.r, -self.s())
    }

    /// 60° counter-clockwise rotation.
    pub fn rotate_ccw(self) -> Self {
        Hex::new(-self.s(), -self.q)
    }

    /// All hexes at exactly `radius` steps from `center` (ordered).
    pub fn ring(center: Hex, radius: u32) -> Vec<Hex> {
        if radius == 0 {
            return vec![center];
        }
        let mut hex = center + DIRECTIONS[4] * radius as i32;
        let mut ring = Vec::with_capacity(6 * radius as usize);
        for dir in DIRECTIONS {
            for _ in 0..radius {
                ring.push(hex);
                hex = hex + dir;
            }
        }
        ring
    }

    /// All hexes within `radius` steps from `center`, center first.
    pub fn spiral(center: Hex, radius: u32) -> Vec<Hex> {
        let mut result = Vec::new();
        for k in 0..=radius {
            result.extend(Hex::ring(center, k));
        }
        result
    }
}

/// The six unit-step directions in axial coordinates.
pub const DIRECTIONS: [Hex; 6] = [
    Hex::new(1, 0),
    Hex::new(0, 1),
    Hex::new(-1, 1),
    Hex::new(-1, 0),
    Hex::new(0, -1),
    Hex::new(1, -1),
];

impl Add for Hex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Hex::new(self.q + rhs.q, self.r + rhs.r)
    }
}

impl Sub for Hex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Hex::new(self.q - rhs.q, self.r - rhs.r)
    }
}

impl Mul<i32> for Hex {
    type Output = Self;
    fn mul(self, k: i32) -> Self {
        Hex::new(self.q * k, self.r * k)
    }
}

impl Neg for Hex {
    type Output = Self;
    fn neg(self) -> Self {
        Hex::new(-self.q, -self.r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_size() {
        for r in 0..5 {
            let ring = Hex::ring(Hex::new(0, 0), r);
            let expected = if r == 0 { 1 } else { 6 * r as usize };
            assert_eq!(ring.len(), expected, "ring radius {r}");
        }
    }

    #[test]
    fn ring_distance() {
        let center = Hex::new(2, -1);
        for r in 0..5 {
            for h in Hex::ring(center, r) {
                assert_eq!(center.distance(h), r as i32, "radius {r} hex {h:?}");
            }
        }
    }

    #[test]
    fn neighbor_distance_one() {
        let h = Hex::new(3, -2);
        for n in h.neighbors() {
            assert_eq!(h.distance(n), 1);
        }
    }

    #[test]
    fn rotation_roundtrip() {
        let h = Hex::new(3, -1);
        let mut x = h;
        for _ in 0..6 {
            x = x.rotate_cw();
        }
        assert_eq!(x, h);
    }

    #[test]
    fn directions_are_ccw() {
        // DIRECTIONS is in CCW order: rotate_cw steps to the next direction,
        // rotate_ccw steps to the previous one.
        for i in 0..6 {
            assert_eq!(DIRECTIONS[i].rotate_cw(), DIRECTIONS[(i + 1) % 6]);
            assert_eq!(DIRECTIONS[i].rotate_ccw(), DIRECTIONS[(i + 5) % 6]);
        }
    }
}
