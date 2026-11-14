// SPDX-License-Identifier: AGPL-3.0-only
//! Literal traversal inventory and named entry refusals precede every numerical read.

use super::{Setup, TestResult, operation, prelude};
use crate::{ApplicationEvaluation, ApplicationRequest};

/// Two cells each cost two components times (three loads + three stores + twenty invocation
/// units), plus two visits per cell. Four owner bindings, two word visits and one entry unit
/// give 115. The neighbour 114 must refuse before any of the twelve numerical loads.
/// ´test:unit:loan-projected-affine-work-boundary´
#[test]
fn loan_projected_affine_work_boundary() -> TestResult {
    boundary::<f32>()?;
    boundary::<f64>()
}

fn boundary<P: nrz_scalar::Scalar + 'static>() -> TestResult {
    for available in [114, 115] {
        let Setup {
            mut heap,
            mut account,
            mut request,
            ..
        } = super::super::super::setup::<P>()?;
        let prelude = prelude::<P>(1, &mut account.ledger)?;
        operation(&mut request, &prelude)?;
        let owner = match heap.request_projected_affine(request, &mut account) {
            ApplicationRequest::ApplicationReady(owner) => owner,
            other @ ApplicationRequest::ApplicationRefused(..) => {
                return Err(format!("{other:?}").into());
            }
        };
        let spend = account
            .limits()
            .work
            .checked_sub(account.usage().work)
            .and_then(|value| value.checked_sub(available))
            .ok_or("remaining Work")?;
        account.ledger.work(spend, crate::loan::AT)?;
        crate::loan::fault::reset_reads();
        let result = owner.evaluate(&mut account);
        if available == 115 {
            assert!(matches!(
                result,
                ApplicationEvaluation::ApplicationCandidate(_)
            ));
            assert_eq!(crate::loan::fault::reads(), 12);
        } else {
            assert!(matches!(
                result,
                ApplicationEvaluation::ApplicationNotStarted(..)
            ));
            assert_eq!(crate::loan::fault::reads(), 0);
        }
    }
    Ok(())
}

/// A binary64 prelude cannot authorize a binary32 request even for identical exact definitions.
/// ´test:unit:loan-projected-affine-format-refusal´
#[test]
fn loan_projected_affine_format_refusal() -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = super::super::super::setup::<f32>()?;
    let prelude = prelude::<f64>(1, &mut account.ledger)?;
    operation(&mut request, &prelude)?;
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    assert!(matches!(
        heap.request_projected_affine(request, &mut account),
        ApplicationRequest::ApplicationRefused(
            crate::ApplicationError::Loan(crate::LoanError::UnsupportedContract),
            _
        )
    ));
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 2]
    );
    drop(injection);
    Ok(())
}

/// Fixed owners contain two three-scalar coefficients, two shared pointers and one Rc header:
/// binary32 24+16+16=56; binary64 48+16+16=80. Export staging is 65536+16384,
/// plus the separately retained pi owner (96/128) and new owner: 82072/82128.
/// ´test:unit:loan-affine-prelude-byte-boundaries´
#[test]
fn loan_affine_prelude_byte_boundaries() -> TestResult {
    prelude_bytes::<f32>(56, 82_072)?;
    prelude_bytes::<f64>(80, 82_128)
}

fn prelude_bytes<P: nrz_scalar::Scalar + 'static>(owned: u64, staged: u64) -> TestResult {
    use nrz_bindings::{Budget, Descriptor, Limits, LimitsOverride};
    use nrz_scalar::{ArithmeticContext, LiteralLimits};
    use std::rc::Rc;
    assert_eq!(
        u64::try_from(crate::AffinePrelude::<P>::owned_bytes())?,
        owned
    );
    let context = ArithmeticContext::new()?;
    let definitions = Rc::new([
        crate::ExactCoefficient::unit(&Descriptor::ratio())?,
        crate::ExactCoefficient::unit(&Descriptor::ratio())?,
    ]);
    let circle = super::circle::<P>(&context, &mut Budget::new(Limits::default())?)?;
    for staging in [false, true] {
        for short in [false, true] {
            let mut limits = Limits::default();
            if staging {
                limits.staged_payload_bytes = staged - u64::from(short);
            } else {
                limits.output_bytes = owned - u64::from(short);
            }
            let mut budget = Budget::new(limits)?;
            let result = crate::AffinePrelude::new(
                Rc::clone(&definitions),
                &circle,
                &LimitsOverride {
                    staged_payload_bytes: Some(65_536),
                    output_bytes: Some(16_384),
                    ..LimitsOverride::default()
                },
                LiteralLimits::default(),
                &context,
                &mut budget,
            );
            assert_eq!(result.is_ok(), !short);
            if short {
                assert_eq!(budget.usage().work, 4);
            }
        }
    }
    Ok(())
}

/// Each exact integer-one spelling costs two C(1)=32 rational-format charges. Native
/// lowering costs eight scans, two digit operations, one displacement and one coefficient
/// enclosure unit. Two coefficients plus four owner-admission units total 156, not a measured pin.
/// ´test:unit:loan-affine-prelude-work-boundary´
#[test]
fn loan_affine_prelude_work_boundary() -> TestResult {
    prelude_work::<f32>()?;
    prelude_work::<f64>()
}

fn prelude_work<P: nrz_scalar::Scalar + 'static>() -> TestResult {
    use nrz_bindings::{Budget, Limits, LimitsOverride};
    use nrz_scalar::{ArithmeticContext, LiteralLimits};
    let context = ArithmeticContext::new()?;
    let circle = super::circle::<P>(&context, &mut Budget::new(Limits::default())?)?;
    let definitions = super::definitions(0)?;
    for available in [155, 156] {
        let mut budget = Budget::new(Limits {
            work: available,
            ..Limits::default()
        })?;
        let result = crate::AffinePrelude::new(
            std::rc::Rc::clone(&definitions),
            &circle,
            &LimitsOverride {
                staged_payload_bytes: Some(65_536),
                output_bytes: Some(16_384),
                ..LimitsOverride::default()
            },
            LiteralLimits::default(),
            &context,
            &mut budget,
        );
        assert_eq!(result.is_ok(), available == 156);
        assert_eq!(budget.usage().work, available);
    }
    Ok(())
}
