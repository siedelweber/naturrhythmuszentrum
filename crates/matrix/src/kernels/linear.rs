// SPDX-License-Identifier: AGPL-3.0-only
//! Numerical affine schedules contain extents, exact coefficients and zero obligations.

use super::lowering::CoefficientValue;
use crate::Extent;
use nrz_scalar::LiteralLimits;

/// Complete numerical data for the fixed affine array law.
#[derive(Clone, Debug)]
pub(crate) struct LinearSchedule {
    /// Numerical input rectangle.
    pub(crate) input: Extent,
    /// Numerical output rectangle.
    pub(crate) output: Extent,
    /// Numerical coefficient matrix rectangle.
    pub(crate) operator: Extent,
    /// Required exact numerical zero entries of the coefficient array.
    pub(crate) zeros: Vec<bool>,
    /// Exact coefficient definitions in logical column order.
    pub(crate) coefficients: Vec<CoefficientValue>,
    /// Exact affine offset definitions in output order.
    pub(crate) offsets: Vec<CoefficientValue>,
    /// Explicit exact-coefficient import limits.
    pub(crate) literals: LiteralLimits,
}

/// Original native cell incidence and its checked scalar coefficient owner remain affine-only.
#[derive(Clone, Debug)]
pub(crate) struct ProjectedAffine {
    /// Immutable original source-to-target incidence.
    pub(crate) map: crate::EntryMap,
    /// Original exact definitions and native enclosure owner.
    pub(crate) prelude: super::affine::Prelude,
}
