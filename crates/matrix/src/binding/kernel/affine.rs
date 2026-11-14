// SPDX-License-Identifier: AGPL-3.0-only
//! Projected scalar affine laws bind original typed constants to original homogeneous meanings.

use super::{KernelDraft, regions};
use crate::kernels::contract::{Recipe, work};
use crate::{
    AffinePrelude, CheckedShape, EntryMap, KernelError, OperationDraft, OperationError,
    RegionContract,
};
use nrz_scalar::Scalar;

impl OperationDraft {
    /// Bind a checked native prelude to one original cell projection.
    ///
    /// The scale and offset are exact typed definitions; no heap coefficient or
    /// caller-supplied numerical centre can enter this law.
    /// # Errors
    /// Refuses foreign snapshots, nonnative maps and incompatible coefficient meanings.
    pub fn map_projected_affine<P: Scalar + 'static>(
        input: CheckedShape,
        output: CheckedShape,
        map: EntryMap,
        prelude: &AffinePrelude<P>,
    ) -> Result<Self, OperationError> {
        input.same_snapshot(&output)?;
        let native = map.product().is_some() || map.selection().is_some();
        let input_mismatch = input.extent() != map.source();
        let output_mismatch = output.extent() != map.target();
        if !native
            || input_mismatch
            || output_mismatch
            || map.source().columns() != map.target().columns()
        {
            return Err(KernelError::Contract.into());
        }
        let source = RegionContract::check(input.clone())?;
        let output = RegionContract::check(output)?;
        let prelude = prelude.erase();
        let definitions = prelude.definitions();
        let expected = input
            .declarations()
            .registry()
            .binary(
                nrz_bindings::BinaryOp::Multiply,
                definitions[0].descriptor(),
                source.descriptor(),
            )
            .map_err(crate::ShapeError::from)?;
        if &expected != output.descriptor() || definitions[1].descriptor() != output.descriptor() {
            return Err(KernelError::Contract.into());
        }
        Ok(super::draft::kernel(KernelDraft {
            reciprocal: None,
            work: work(output.shape().extent().entries(), 20)?,
            inputs: regions(vec![input])?,
            output,
            recipe: Recipe::ProjectedAffine(Box::new(crate::kernels::linear::ProjectedAffine {
                map,
                prelude,
            })),
        }))
    }
}
