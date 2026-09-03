//! Hamilton quaternions, stored as `(w, x, y, z)` to match the Python stack.

use glam::{Vec3, Vec4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Q(pub Vec4);

impl Q {
    pub const IDENTITY: Self = Self(Vec4::new(1.0, 0.0, 0.0, 0.0));

    pub fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self(Vec4::new(w, x, y, z))
    }

    pub fn from_xyzw(v: Vec4) -> Self {
        Self(v)
    }

    pub fn w(self) -> f32 {
        self.0.x
    }
    pub fn x(self) -> f32 {
        self.0.y
    }
    pub fn y(self) -> f32 {
        self.0.z
    }
    pub fn z(self) -> f32 {
        self.0.w
    }

    pub fn imag(self) -> Vec3 {
        Vec3::new(self.x(), self.y(), self.z())
    }

    pub fn norm2(self) -> f32 {
        self.0.length_squared()
    }

    pub fn norm(self) -> f32 {
        self.0.length()
    }

    pub fn conjugate(self) -> Self {
        Self::new(self.w(), -self.x(), -self.y(), -self.z())
    }

    pub fn normalize(self) -> Self {
        let n = self.norm();
        if n < 1e-12 {
            Self::IDENTITY
        } else {
            Self(self.0 / n)
        }
    }

    pub fn mul(self, other: Self) -> Self {
        let (w1, x1, y1, z1) = (self.w(), self.x(), self.y(), self.z());
        let (w2, x2, y2, z2) = (other.w(), other.x(), other.y(), other.z());
        Self::new(
            w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
            w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
            w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
            w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
        )
    }

    pub fn from_axis_angle(axis: Vec3, theta: f32) -> Self {
        let n = axis.length();
        if n < 1e-12 {
            return Self::IDENTITY;
        }
        let axis = axis / n;
        let half = theta * 0.5;
        let s = half.sin();
        Self::new(half.cos(), axis.x * s, axis.y * s, axis.z * s)
    }

    /// Unit quaternion for a pure right-phase (rotation in the 1–k plane).
    pub fn phase_unit(theta: f32) -> Self {
        Self::new((theta * 0.5).cos(), 0.0, 0.0, (theta * 0.5).sin())
    }

    /// Chordal distance on S³; optionally identify q ~ −q.
    pub fn chordal_distance(self, other: Self, identify_antipodes: bool) -> f32 {
        let d = (self.0 - other.0).length();
        if identify_antipodes {
            d.min((self.0 + other.0).length())
        } else {
            d
        }
    }

    pub fn as_vec4(self) -> Vec4 {
        self.0
    }
}

impl std::ops::Mul for Q {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Q::mul(self, rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i_times_j_is_k() {
        let i = Q::new(0.0, 1.0, 0.0, 0.0);
        let j = Q::new(0.0, 0.0, 1.0, 0.0);
        let k = i * j;
        assert!((k.w()).abs() < 1e-6);
        assert!((k.x()).abs() < 1e-6);
        assert!((k.y()).abs() < 1e-6);
        assert!((k.z() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn j_times_i_is_minus_k() {
        let i = Q::new(0.0, 1.0, 0.0, 0.0);
        let j = Q::new(0.0, 0.0, 1.0, 0.0);
        let p = j * i;
        assert!((p.z() + 1.0).abs() < 1e-6);
    }
}
