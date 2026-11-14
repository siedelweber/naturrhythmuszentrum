// SPDX-License-Identifier: AGPL-3.0-only
//! Full and empty coverage, protected native owners and repairable original-law captures.

use super::{Setup, TestResult, candidate, operation, prelude, publish};
use crate::{ApplicationEvaluation, ApplicationPublication, ApplicationRequest, OwnedApplication};
use nrz_scalar::Scalar;

/// Complete selections and vacuous zero-capacity targets have no invented cells or loads.
/// ´test:unit:loan-projected-affine-full-and-empty´
#[test]
fn loan_projected_affine_full_and_empty() -> TestResult {
    full::<f32>()?;
    full::<f64>()
}

fn full<P: Scalar + 'static>() -> TestResult {
    for cells in [&[][..], &[0, 1, 4][..]] {
        let Setup {
            mut heap,
            mut account,
            mut request,
            demand,
        } = super::super::super::setup_selection::<P>(cells, cells.len())?;
        let coefficients = prelude::<P>(1, &mut account.ledger)?;
        operation(&mut request, &coefficients)?;
        drop(coefficients);
        let owner = ready(heap.request_projected_affine(request, &mut account))?;
        crate::loan::fault::reset_reads();
        let owner = candidate(owner, &mut account)?;
        assert_eq!(crate::loan::fault::reads(), cells.len() * 6);
        let tile = publish(&mut heap, &mut account, owner)?;
        let reader = tile.reader(&heap, &mut account)?;
        for cell in 0..cells.len() {
            assert!(reader.contains(cell, &mut account)?);
        }
        drop(tile);
        heap.retire_mask(demand, &mut account.ledger)?;
        super::super::super::retire_copy_layout(&mut heap, &mut account, demand.layout())?;
        let _residence = heap.residence(&mut account, 100_000)?;
        super::super::super::empty(&mut heap, &mut account)?;
    }
    Ok(())
}

/// Definition substitution, companion swaps, stale incidence and forged scope refuse without loads.
/// ´test:unit:loan-projected-affine-capture-repair´
#[test]
fn loan_projected_affine_capture_repair() -> TestResult {
    for publication in [false, true] {
        for mutation in 0..4 {
            repair::<f32>(publication, mutation)?;
            repair::<f64>(publication, mutation)?;
        }
    }
    Ok(())
}

fn repair<P: Scalar + 'static>(publication: bool, mutation: u8) -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = super::super::super::setup::<P>()?;
    let coefficients = prelude::<P>(1, &mut account.ledger)?;
    operation(&mut request, &coefficients)?;
    let alternative = prelude::<P>(0, &mut account.ledger)?;
    let crate::binding::NativeRecipe::Kernel(law) = &request.operation.recipe else {
        return Err("kernel".into());
    };
    let crate::kernels::contract::Recipe::ProjectedAffine(affine) = &law.recipe else {
        return Err("affine".into());
    };
    let alternate = crate::OperationDraft::map_projected_affine(
        request.operation.inputs()[0].shape().clone(),
        request.operation.outputs()[0].shape().clone(),
        affine.map.clone(),
        &alternative,
    )?
    .check(crate::ExecutionLimits::default())?;
    let mut owner = ready(heap.request_projected_affine(request, &mut account))?;
    if publication {
        owner = candidate(owner, &mut account)?;
    }
    let ticket = owner.ticket();
    let original = owner.core.evidence.operation.clone();
    let attachments = owner.core.evidence.attachments;
    let demand = owner.core.evidence.demand;
    let scope = owner.core.evidence.error_scope;
    match mutation {
        0 => owner.core.evidence.operation = alternate,
        1 => owner.core.evidence.attachments.swap(1, 2),
        2 => owner.core.evidence.demand.layout = owner.core.evidence.source_identity.layout,
        3 => owner.core.evidence.error_scope = crate::NumericalErrorScope::HostInput,
        _ => return Err("mutation".into()),
    }
    owner = refused(owner, publication, &mut heap, &mut account)?;
    assert_eq!(owner.ticket(), ticket);
    owner.core.evidence.operation = original;
    owner.core.evidence.attachments = attachments;
    owner.core.evidence.demand = demand;
    owner.core.evidence.error_scope = scope;
    if !publication {
        owner = candidate(owner, &mut account)?;
    }
    let _tile = publish(&mut heap, &mut account, owner)?;
    Ok(())
}

fn ready<P: Scalar>(result: ApplicationRequest<P>) -> TestResult<OwnedApplication<P>> {
    match result {
        ApplicationRequest::ApplicationReady(owner) => Ok(owner),
        other @ ApplicationRequest::ApplicationRefused(..) => Err(format!("{other:?}").into()),
    }
}

fn refused<P: Scalar + 'static>(
    owner: OwnedApplication<P>,
    publication: bool,
    heap: &mut crate::Heap,
    account: &mut crate::ExecutionAccount,
) -> TestResult<OwnedApplication<P>> {
    crate::loan::fault::reset_reads();
    crate::loan::fault::reset_stores();
    let injection = crate::loan::fault::Injection::after(0);
    let owner = if publication {
        match heap.publish_application(owner, account) {
            ApplicationPublication::ApplicationRepair(_, owner) => owner,
            other => return Err(format!("{other:?}").into()),
        }
    } else {
        match owner.evaluate(account) {
            ApplicationEvaluation::ApplicationNotStarted(_, owner) => owner,
            other => return Err(format!("{other:?}").into()),
        }
    };
    assert_eq!(
        [
            crate::loan::fault::reads(),
            crate::loan::fault::stores(),
            crate::loan::fault::Injection::attempts()
        ],
        [0; 3]
    );
    drop(injection);
    Ok(owner)
}

/// Current affine results compose with propagated error; a reclaimed host makes reuse stale.
/// ´test:unit:loan-projected-affine-current-lineage´
#[test]
fn loan_projected_affine_current_lineage() -> TestResult {
    for stale in [false, true] {
        lineage::<f32>(stale)?;
        lineage::<f64>(stale)?;
    }
    Ok(())
}

fn lineage<P: Scalar + 'static>(stale: bool) -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = super::super::super::setup::<P>()?;
    let coefficients = prelude::<P>(1, &mut account.ledger)?;
    operation(&mut request, &coefficients)?;
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    let owner = candidate(owner, &mut account)?;
    let tile = publish(&mut heap, &mut account, owner)?;
    let mut request = super::super::super::next_copy(&mut heap, &mut account, tile.into_pin())?;
    let shift = prelude::<P>(0, &mut account.ledger)?;
    operation(&mut request, &shift)?;
    if stale {
        assert!(matches!(
            heap.residence(&mut account, 100_000)?,
            crate::Residence::Pending { .. }
        ));
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let request = match heap.request_projected_affine(request, &mut account) {
            ApplicationRequest::ApplicationRefused(
                crate::ApplicationError::Loan(crate::LoanError::StaleDependency),
                request,
            ) => request,
            other => return Err(format!("{other:?}").into()),
        };
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
        assert!(
            request
                .input
                .historical_reader(&mut account)?
                .read_component(crate::HostChannel::Error, 2, 1, &mut account)?
                > P::ZERO
        );
        return Ok(());
    }
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    let before = account.usage().work;
    let owner = candidate(owner, &mut account)?;
    // The same 115-unit traversal plus two immediate-ancestor seal checks.
    assert_eq!(account.usage().work - before, 117);
    let tile = publish(&mut heap, &mut account, owner)?;
    let reader = tile.reader(&heap, &mut account)?;
    assert_eq!(
        reader.numerical_error_scope(),
        crate::NumericalErrorScope::PropagatedInput
    );
    for (cell, original) in [(0, 1), (2, 4)] {
        for component in 0..2 {
            let readings = [
                crate::HostChannel::Center,
                crate::HostChannel::Total,
                crate::HostChannel::Error,
            ]
            .map(|channel| {
                reader
                    .read_component(channel, cell, component, &mut account)
                    .map(Scalar::to_f64)
            });
            let [center, total, error] = readings;
            super::oracle::composed::<P>([center?, total?, error?], original, component)?;
        }
    }
    Ok(())
}

/// Affine propagation after local arithmetic cannot acquire host-chain numerical-error authority.
/// ´test:unit:loan-projected-affine-local-error-scope´
#[test]
fn loan_projected_affine_local_error_scope() -> TestResult {
    local_scope::<f32>()?;
    local_scope::<f64>()
}

fn local_scope<P: Scalar + 'static>() -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = super::super::super::setup::<P>()?;
    super::super::square(&mut request)?;
    let owner = ready(heap.request_projected_elementwise(request, &mut account))?;
    let owner = candidate(owner, &mut account)?;
    let tile = publish(&mut heap, &mut account, owner)?;
    let first = tile.reader(&heap, &mut account)?;
    assert_eq!(
        first.numerical_error_scope(),
        crate::NumericalErrorScope::LocalOperation
    );
    let prior = first.read_component(crate::HostChannel::Error, 0, 0, &mut account)?;
    let mut request = super::super::super::next_copy(&mut heap, &mut account, tile.into_pin())?;
    let shift = prelude::<P>(0, &mut account.ledger)?;
    operation(&mut request, &shift)?;
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    let owner = candidate(owner, &mut account)?;
    let tile = publish(&mut heap, &mut account, owner)?;
    let reader = tile.reader(&heap, &mut account)?;
    assert_eq!(
        reader.numerical_error_scope(),
        crate::NumericalErrorScope::PropagatedLocal
    );
    let center = reader
        .read_component(crate::HostChannel::Center, 0, 0, &mut account)?
        .to_f64();
    let error = reader
        .read_component(crate::HostChannel::Error, 0, 0, &mut account)?
        .to_f64();
    assert!(error >= prior.to_f64());
    assert!(error < 0.01);
    assert!(crate::kernel_oracle::Rational::new(122, 1)?.covered(center, error)?);
    Ok(())
}
