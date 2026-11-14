// SPDX-License-Identifier: AGPL-3.0-only
//! Exact power-of-two normalization keeps tiny and large independent rational oracles bounded.

use super::{Setup, TestResult, candidate, operation, publish};
use crate::{ApplicationRequest, Coverage, HostChannel};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Precision, Scalar};

/// Tiny and large pi images remain enclosed; overflowing candidates refuse their complete cell.
/// ´test:unit:loan-projected-affine-extreme-domains´
#[test]
fn loan_projected_affine_extreme_domains() -> TestResult {
    for coverage in [Coverage::RepresentedInputs, Coverage::DeclaredBounds] {
        domains::<f32>(coverage)?;
        domains::<f64>(coverage)?;
    }
    Ok(())
}

fn domains<P: Scalar + 'static>(coverage: Coverage) -> TestResult {
    let powers = [-100, 100]
        .map(|exponent| P::parse_nearest(&2.0_f64.powi(exponent).to_string()).ok_or("power"));
    let [small, large] = powers;
    let [small, large] = [small?, large?];
    let overflow = P::parse_nearest(match P::PRECISION {
        Precision::F32 => "3e38",
        Precision::F64 => "1e308",
    })
    .ok_or("large")?;
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = super::super::unary::domain_setup(
        [[small; 2], [large; 2], [overflow; 2]],
        [
            [small / P::from_u8(4); 2],
            [large / P::from_u8(4); 2],
            [P::ZERO; 2],
        ],
        coverage,
    )?;
    let definitions = super::definitions(1)?;
    let definitions = std::rc::Rc::new([
        definitions[0].clone(),
        crate::ExactCoefficient::zero(&nrz_bindings::Descriptor::ratio())?,
    ]);
    let context = ArithmeticContext::new()?;
    let circle = super::circle::<P>(&context, &mut account.ledger)?;
    let prelude = crate::AffinePrelude::new(
        definitions,
        &circle,
        &nrz_bindings::LimitsOverride {
            staged_payload_bytes: Some(65_536),
            output_bytes: Some(16_384),
            ..nrz_bindings::LimitsOverride::default()
        },
        LiteralLimits::default(),
        &context,
        &mut account.ledger,
    )?;
    operation(&mut request, &prelude)?;
    let owner = match heap.request_projected_affine(request, &mut account) {
        ApplicationRequest::ApplicationReady(owner) => owner,
        other @ ApplicationRequest::ApplicationRefused(..) => {
            return Err(format!("{other:?}").into());
        }
    };
    let owner = candidate(owner, &mut account)?;
    let tile = publish(&mut heap, &mut account, owner)?;
    let reader = tile.reader(&heap, &mut account)?;
    assert_eq!(reader.coverage(), coverage);
    for (cell, exponent) in [(0, -100), (1, 100)] {
        assert!(reader.contains(cell, &mut account)?);
        let readings =
            [HostChannel::Center, HostChannel::Total, HostChannel::Error].map(|channel| {
                reader
                    .read_component(channel, cell, 0, &mut account)
                    .map(|value| value.to_f64() * 2.0_f64.powi(-exponent))
            });
        let [center, total, error] = readings;
        super::oracle::normalized::<P>([center?, total?, error?])?;
    }
    assert!(!reader.contains(2, &mut account)?);
    assert!(matches!(
        reader.application_outcome(2, &mut account)?,
        crate::CellOutcome::CellRefused { component: 0, .. }
    ));
    Ok(())
}
