// SPDX-License-Identifier: AGPL-3.0-only
//! Exact scalar affine definitions own one checked native prelude, shared by the tile law.

use crate::{ConversionError, ExactCoefficient, PiConstant};
use nrz_bindings::{Budget, Limits, LimitsOverride, readings::algebra::limits::BudgetKind};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};
use std::{any::Any, fmt::Debug, rc::Rc};

/// One exact scale and offset, lowered together without caller-written numerical coefficients.
#[derive(Clone, Debug)]
pub struct AffinePrelude<P: Scalar>(Rc<Owner<P>>);

/// Fixed numerical storage retains the original exact definitions and shared pi certificate.
#[derive(Debug)]
pub(crate) struct Owner<P: Scalar> {
    /// Exact typed scale followed by exact typed offset.
    definitions: Rc<[ExactCoefficient; 2]>,
    /// Independently enclosed original constants in the execution format.
    pub(crate) coefficients: [super::lowering::Coefficient<P>; 2],
    /// Original shared pi materialization, including its arithmetic revision.
    pi: PiConstant<P>,
}

impl<P: Scalar + 'static> AffinePrelude<P> {
    /// Enclose the two original definitions under explicit cumulative policies.
    ///
    /// This constructor accepts exact typed definitions only. Rational factors and
    /// supported pi factors use the same bounded exporter and native decoder.
    /// # Errors
    /// Refuses unsupported expressions, arithmetic mismatches and exhausted resources.
    pub fn new(
        definitions: Rc<[ExactCoefficient; 2]>,
        pi: &PiConstant<P>,
        exact: &LimitsOverride,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, ConversionError> {
        if pi.arithmetic_revision() != context.revision() {
            return Err(ConversionError::Contract);
        }
        budget.with_work_phase(
            "matrix affine prelude",
            "coefficient owner admission",
            |budget| {
                budget
                    .work(
                        4,
                        nrz_bindings::readings::algebra::Location::new(
                            "affine coefficient prelude",
                        ),
                    )
                    .map_err(nrz_bindings::MapError::from)?;
                reserve::<P>(pi, exact, budget)
            },
        )?;
        let scale = super::lowering::checked_definition(
            &definitions[0],
            pi,
            exact,
            literals,
            context,
            budget,
        )?;
        let offset = super::lowering::checked_definition(
            &definitions[1],
            pi,
            exact,
            literals,
            context,
            budget,
        )?;
        Ok(Self(Rc::new(Owner {
            definitions,
            coefficients: [scale, offset],
            pi: pi.clone(),
        })))
    }

    /// New numerical owner and reference header; exact and pi owners remain shared.
    #[must_use]
    pub const fn owned_bytes() -> usize {
        size_of::<Owner<P>>() + 2 * size_of::<usize>()
    }

    /// Erasure retains the original allocation and never widens numerical evidence.
    pub(crate) fn erase(&self) -> Prelude {
        Prelude(self.0.clone())
    }
}

/// Read-only inspection preserves the exact original allocation across closed native formats.
trait View: Debug + Any {
    /// Native downcast is checked once per tile, before any numerical load.
    fn any(&self) -> &dyn Any;
    /// Original typed exact definitions.
    fn definitions(&self) -> &[ExactCoefficient; 2];
    /// Original fixed numerical allocation size.
    fn owned_bytes(&self) -> usize;
    /// Shared pi allocation and its independent destruction entitlement.
    fn pi_owner(&self) -> NativeOwner;
}
impl<P: Scalar + 'static> View for Owner<P> {
    fn any(&self) -> &dyn Any {
        self
    }
    fn definitions(&self) -> &[ExactCoefficient; 2] {
        &self.definitions
    }
    fn owned_bytes(&self) -> usize {
        AffinePrelude::<P>::owned_bytes()
    }
    fn pi_owner(&self) -> NativeOwner {
        NativeOwner {
            witness: self.pi.allocation_witness(),
            bytes: self.pi.owned_bytes(),
            cleanup: 1,
        }
    }
}

/// Format-erased retaining edge inside the original immutable operation.
#[derive(Clone, Debug)]
pub(crate) struct Prelude(Rc<dyn View>);
impl Prelude {
    /// Original descriptor meanings survive numerical lowering unchanged.
    pub(crate) fn definitions(&self) -> &[ExactCoefficient; 2] {
        self.0.definitions()
    }
    /// A foreign format or arithmetic revision cannot reuse a numerical prelude.
    pub(crate) fn bind<P: Scalar + 'static>(
        &self,
        context: &ArithmeticContext,
    ) -> Result<&Owner<P>, ConversionError> {
        let owner = self
            .0
            .any()
            .downcast_ref::<Owner<P>>()
            .ok_or(ConversionError::Contract)?;
        if owner.pi.arithmetic_revision() != context.revision() {
            return Err(ConversionError::Contract);
        }
        Ok(owner)
    }
}

/// Reserve the fixed owner and bounded export scratch before constructing any numerical owner.
fn reserve<P: Scalar + 'static>(
    pi: &PiConstant<P>,
    exact: &LimitsOverride,
    budget: &Budget,
) -> Result<(), ConversionError> {
    let limits = Limits::default()
        .resolve(exact)
        .map_err(nrz_bindings::MapError::from)?;
    let owned = AffinePrelude::<P>::owned_bytes();
    let staged = limits
        .staged_payload_bytes
        .checked_add(limits.output_bytes)
        .and_then(|value| usize::try_from(value).ok())
        .and_then(|value| value.checked_add(pi.owned_bytes()))
        .and_then(|value| value.checked_add(owned))
        .ok_or(ConversionError::ResourceLimit)?;
    for (kind, bytes) in [
        (BudgetKind::StagedPayloadBytes, staged),
        (BudgetKind::OutputBytes, owned),
    ] {
        budget
            .check(
                kind,
                u64::try_from(bytes).map_err(|_| ConversionError::ResourceLimit)?,
                nrz_bindings::readings::algebra::Location::new("affine coefficient prelude"),
            )
            .map_err(nrz_bindings::MapError::from)?;
    }
    Ok(())
}

impl<P: Scalar> Owner<P> {
    /// Enclose the nominal law and propagate total and numerical-only input radii separately.
    pub(crate) fn evaluate(
        &self,
        values: [P; 3],
        context: &ArithmeticContext,
    ) -> Result<super::arithmetic::Entry<P>, super::KernelError> {
        use nrz_accuracy::Nonnegative;
        let [scale, offset] = self.coefficients;
        let [value, total, prior] = values;
        let candidate = scale.center * value + offset.center;
        let nominal = scale
            .enclosure
            .mul(super::arithmetic::singleton(value)?, context)?
            .add(offset.enclosure, context)?;
        let magnitude = Nonnegative::new(scale.enclosure.abs().upper().get())?;
        let propagated = magnitude.scale(Nonnegative::new(total)?, context)?;
        let mut result = super::arithmetic::finish(candidate, nominal, propagated, context)?;
        result.error = magnitude
            .scale(Nonnegative::new(prior)?, context)?
            .add(Nonnegative::new(result.error)?, context)?
            .get();
        Ok(result)
    }
}

/// An immutable numerical allocation's identity, without retaining or upgrading the owner.
#[derive(Clone, Debug)]
pub(crate) struct NativeOwner {
    /// Expiration and exact shared-allocation identity only.
    pub(crate) witness: std::rc::Weak<dyn Any>,
    /// Actual fixed owner plus reference-header bytes.
    pub(crate) bytes: usize,
    /// Allocation destruction and strong-edge releases paid once on managed adoption.
    pub(crate) cleanup: usize,
}
impl Prelude {
    /// The fixed coefficient owner and its shared pi region are distinct physical allocations.
    pub(crate) fn owners(&self) -> [NativeOwner; 2] {
        let owner: Rc<dyn Any> = self.0.clone();
        [
            NativeOwner {
                witness: Rc::downgrade(&owner),
                bytes: self.0.owned_bytes(),
                cleanup: 3,
            },
            self.0.pi_owner(),
        ]
    }
}
