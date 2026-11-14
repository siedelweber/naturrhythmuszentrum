// SPDX-License-Identifier: AGPL-3.0-only
//! Only the affine entry point admits its exact prelude, and unsupported definitions stay exact refusals.

use super::{Setup, TestResult, operation, prelude};
use crate::{ApplicationError, ApplicationRequest, LoanError};
use nrz_scalar::Scalar;

/// Ordinary, movement, projected-unary and chained entry points cannot reinterpret the affine law.
/// ´test:unit:loan-projected-affine-entry-refusals´
#[test]
fn loan_projected_affine_entry_refusals() -> TestResult {
    entries::<f32>()?;
    entries::<f64>()
}

fn entries<P: Scalar + 'static>() -> TestResult {
    for entry in 0..4 {
        let Setup {
            mut heap,
            mut account,
            mut request,
            ..
        } = super::super::super::setup::<P>()?;
        let prelude = prelude::<P>(1, &mut account.ledger)?;
        operation(&mut request, &prelude)?;
        let ticket = request.input.ticket()?;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result = match entry {
            0 => heap.request_pointwise_square(request, &mut account),
            1 => heap.request_mapped_copy(request, &mut account),
            2 => heap.request_projected_elementwise(request, &mut account),
            3 => heap.request_chained_pointwise_square(request, &mut account),
            _ => return Err("entry".into()),
        };
        let request = match result {
            ApplicationRequest::ApplicationRefused(
                ApplicationError::Loan(LoanError::UnsupportedContract),
                request,
            ) => request,
            other => return Err(format!("{other:?}").into()),
        };
        assert_eq!(request.input.ticket()?, ticket);
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        assert!(matches!(
            heap.request_projected_affine(request, &mut account),
            ApplicationRequest::ApplicationReady(_)
        ));
    }
    Ok(())
}

/// The prelude grammar refuses a sum containing pi instead of inserting approximate coefficients.
/// ´test:unit:loan-affine-prelude-unsupported-definition´
#[test]
fn loan_affine_prelude_unsupported_definition() -> TestResult {
    unsupported::<f32>()?;
    unsupported::<f64>()
}

fn unsupported<P: Scalar + 'static>() -> TestResult {
    use nrz_bindings::{Budget, Conversion, Descriptor, ExactConstant, Limits, LimitsOverride};
    let mut setup = Budget::new(Limits::default())?;
    let context = nrz_scalar::ArithmeticContext::new()?;
    let circle = super::circle::<P>(&context, &mut setup)?;
    let pair = super::definitions(1)?;
    let sum = pair[0]
        .exact_map()
        .scale
        .add(&ExactConstant::one(), &mut setup)?;
    let unsupported = crate::ExactCoefficient::new(
        &Descriptor::ratio(),
        &Conversion {
            scale: sum,
            shift: ExactConstant::zero(),
        },
    )?;
    let definitions = std::rc::Rc::new([unsupported, pair[1].clone()]);
    let mut budget = Budget::new(Limits::default())?;
    assert!(matches!(
        crate::AffinePrelude::new(
            definitions,
            &circle,
            &LimitsOverride {
                staged_payload_bytes: Some(65_536),
                output_bytes: Some(16_384),
                ..LimitsOverride::default()
            },
            nrz_scalar::LiteralLimits::default(),
            &context,
            &mut budget
        ),
        Err(crate::ConversionError::Map(nrz_bindings::MapError::Units(
            nrz_bindings::UnitError::UnsupportedExactConstantExport
        )))
    ));
    assert!(budget.usage().work > 4);
    Ok(())
}
