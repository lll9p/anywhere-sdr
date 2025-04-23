//! Geodetic coordinate transformations and navigation calculations
//! Implements formulas from <http://www.movable-type.co.uk/scripts/latlong.html>
mod coordinates;
#[cfg(test)]
mod tests;
mod traits;
mod transformation;
pub use coordinates::{Azel, Ecef, Location, NavigationTarget, Neu};
pub use traits::LocationMath;
