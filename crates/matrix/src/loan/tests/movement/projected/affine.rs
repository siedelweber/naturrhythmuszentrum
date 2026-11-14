// SPDX-License-Identifier: AGPL-3.0-only
//! Literal source tuples and independent rational images exercise the checked coefficient prelude.

use super::super::{Setup, TestResult, candidate, publish};
use crate::{AffinePrelude, ApplicationRequest, ExactCoefficient, HostChannel, PointwiseRequest};
use nrz_bindings::{Budget, Descriptor, Kind, LimitsOverride, Measure, Role, Scale};
use nrz_scalar::{ArithmeticContext, Finite, LiteralLimits, PiLimits, Precision, Scalar};
use std::rc::Rc;

mod boundaries;
mod domains;
mod lifecycle;
mod oracle;
mod refusals;
mod retention;

/// Use the fixed scalar pi policy already admitted by the conversion evidence.
pub(super) fn prelude<P: Scalar + 'static>(
    kind: u8,
    budget: &mut Budget,
) -> TestResult<AffinePrelude<P>> {
    let context = ArithmeticContext::new()?;
    let circle = circle::<P>(&context, budget)?;
    prelude_with_pi(kind, &circle, &context, budget)
}

/// Distinct coefficient owners may share the same independently retained pi allocation.
pub(super) fn prelude_with_pi<P: Scalar + 'static>(
    kind: u8,
    circle: &crate::PiConstant<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> TestResult<AffinePrelude<P>> {
    Ok(AffinePrelude::new(
        definitions(kind)?,
        circle,
        &LimitsOverride {
            staged_payload_bytes: Some(65_536),
            output_bytes: Some(16_384),
            ..LimitsOverride::default()
        },
        LiteralLimits::default(),
        context,
        budget,
    )?)
}

/// Exact schema operations construct signed and reciprocal factors without numerical constants.
pub(super) fn definitions(kind: u8) -> TestResult<Rc<[ExactCoefficient; 2]>> {
    use nrz_bindings::{Conversion, ExactConstant, Limits};
    let descriptor = Descriptor::new(
        Kind::Ratio,
        Measure::new(
            "pi-affine-unit",
            Scale::pi()?,
            Measure::coherent().offset().clone(),
        )?,
        Role::Difference,
    )?;
    let circle = ExactCoefficient::unit(&descriptor)?;
    let ratio = Descriptor::ratio();
    let one = ExactCoefficient::unit(&ratio)?;
    let mut budget = Budget::new(Limits::default())?;
    let scale = match kind {
        0 => one.clone(),
        1 => circle.clone(),
        2 | 3 => ExactCoefficient::new(
            &ratio,
            &Conversion {
                scale: if kind == 2 {
                    circle.exact_map().scale.negated(&mut budget)?
                } else {
                    ExactConstant::one().div(&circle.exact_map().scale, &mut budget)?
                },
                shift: ExactConstant::zero(),
            },
        )?,
        _ => return Err("coefficient fixture".into()),
    };
    Ok(Rc::new([scale, if kind < 2 { one } else { circle }]))
}

/// Materialize the independent pi owner before testing coefficient-owner reservations.
pub(super) fn circle<P: Scalar + 'static>(
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> TestResult<crate::PiConstant<P>> {
    let (tolerance, work, steps, staged_bytes, output_bytes) = match P::PRECISION {
        Precision::F32 => ("0.0000003", 668, 5, 240, 64),
        Precision::F64 => ("0.0000000000000005", 2054, 11, 256, 80),
    };
    Ok(crate::PiConstant::new(
        Finite::new(P::parse_nearest(tolerance).ok_or("tolerance")?)?,
        PiLimits {
            work,
            steps,
            staged_bytes,
            output_bytes,
        },
        context,
        budget,
    )?)
}

/// Replace only the original law; retain the fixture's independently declared maps and companions.
pub(super) fn operation<P: Scalar, Q: Scalar + 'static>(
    request: &mut PointwiseRequest<P>,
    prelude: &AffinePrelude<Q>,
) -> TestResult {
    let crate::binding::NativeRecipe::Kernel(law) = &request.operation.recipe else {
        return Err("kernel".into());
    };
    let crate::kernels::contract::Recipe::Transform(map) = &law.recipe else {
        return Err("projection".into());
    };
    request.operation = crate::OperationDraft::map_projected_affine(
        request.operation.inputs()[0].shape().clone(),
        request.operation.outputs()[0].shape().clone(),
        map.clone(),
        prelude,
    )?
    .check(crate::ExecutionLimits::default())?;
    Ok(())
}

/// Both formats retain original projected incidence and propagate nonzero input error.
/// ´test:unit:loan-projected-affine-native-images´
#[test]
fn loan_projected_affine_native_images() -> TestResult {
    for kind in 0..4 {
        for product in [false, true] {
            direct::<f32>(kind, product)?;
            direct::<f64>(kind, product)?;
        }
    }
    Ok(())
}

/// Every expected original source cell is literal and independent of the production entry map.
fn direct<P: Scalar + 'static>(kind: u8, product: bool) -> TestResult {
    let (setup, outside) = if product {
        let (setup, outside) = super::super::product_setup::<P>()?;
        (setup, Some(outside))
    } else {
        (
            super::super::setup_layout::<P>(
                &[1, 3, 4],
                4,
                Some(crate::Layout::strided(
                    crate::Extent::new(5, 2)?,
                    16,
                    13,
                    -3,
                    1,
                )?),
            )?,
            None,
        )
    };
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = setup;
    let prelude = prelude::<P>(kind, &mut account.ledger)?;
    operation(&mut request, &prelude)?;
    let owner = match heap.request_projected_affine(request, &mut account) {
        ApplicationRequest::ApplicationReady(owner) => owner,
        other @ ApplicationRequest::ApplicationRefused(..) => {
            return Err(format!("{other:?}").into());
        }
    };
    drop(prelude);
    crate::loan::fault::reset_reads();
    let owner = candidate(owner, &mut account)?;
    assert_eq!(crate::loan::fault::reads(), if product { 36 } else { 12 });
    let tile = publish(&mut heap, &mut account, owner)?;
    let reader = tile.reader(&heap, &mut account)?;
    assert_eq!(
        reader.numerical_error_scope(),
        crate::NumericalErrorScope::PropagatedInput
    );
    let expected = if product {
        vec![(0, 0), (1, 0), (2, 1), (8, 4)]
    } else {
        vec![(0, 1), (2, 4)]
    };
    for (cell, original) in expected {
        for component in 0..2 {
            let values =
                [HostChannel::Center, HostChannel::Total, HostChannel::Error].map(|channel| {
                    reader
                        .read_component(channel, cell, component, &mut account)
                        .map(Scalar::to_f64)
                });
            let [center, total, error] = values;
            oracle::check::<P>([center?, total?, error?], original, component, kind)?;
        }
    }
    if let Some(outside) = outside {
        assert_eq!(
            outside.reader(&heap, &mut account)?.read_component(
                HostChannel::Error,
                4,
                1,
                &mut account
            )?,
            P::from_u8(2)
        );
    }
    Ok(())
}
