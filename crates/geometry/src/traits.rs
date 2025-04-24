/// Common mathematical operations for geodetic coordinate types.
///
/// This trait defines basic vector operations that are implemented by
/// various coordinate types (Location, Ecef, Neu) to enable consistent
/// mathematical operations across different coordinate representations.
pub trait LocationMath {
    /// Calculates the vector magnitude (Euclidean norm).
    ///
    /// Returns the length of the vector represented by this coordinate.
    /// For ECEF coordinates, this is the distance from Earth's center.
    /// For LLH coordinates, this is less meaningful but still available.
    fn norm(&self) -> f64;

    /// Computes the dot product between two vectors.
    ///
    /// # Arguments
    /// * `_rhs` - Another vector of the same type
    ///
    /// # Returns
    /// The scalar dot product of the two vectors
    fn dot_prod(&self, _rhs: &Self) -> f64;

    /// Checks if two coordinates are approximately equal within a tolerance.
    ///
    /// This method is only available in test builds and is used to compare
    /// floating-point coordinate values with an epsilon tolerance to account
    /// for numerical precision issues.
    ///
    /// # Arguments
    /// * `rhs` - Another coordinate to compare with
    /// * `eps` - Epsilon tolerance for floating-point comparison
    ///
    /// # Returns
    /// `true` if all components are within epsilon of each other
    #[cfg(test)]
    fn precise(&self, rhs: &Self, eps: f64) -> bool;
}
