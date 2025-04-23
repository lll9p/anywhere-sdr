/// Common mathematical operations for geodetic types
pub trait LocationMath {
    /// Vector magnitude (Euclidean norm)
    fn norm(&self) -> f64;
    /// Dot product between two vectors
    fn dot_prod(&self, _rhs: &Self) -> f64;
    #[cfg(test)]
    /// Approximate equality check with epsilon tolerance
    fn precise(&self, rhs: &Self, eps: f64) -> bool;
}
