// SPDX-License-Identifier: AGPL-3.0-only
//! Shared coefficient and pi owners remain counted through independent caller retention.

use super::{Setup, TestResult, candidate, operation, prelude, publish};
use crate::{ApplicationRequest, Heap, OwnedApplication};
use nrz_scalar::Scalar;

/// Two affine consumers share exactly one coefficient allocation and one pi allocation.
/// Their 56+96 or 80+128 bytes remain resident after the numerical groups retire.
/// ´test:unit:loan-affine-prelude-managed-retention´
#[test]
fn loan_affine_prelude_managed_retention() -> TestResult {
    retention::<f32>(152)?;
    retention::<f64>(208)
}

fn retention<P: Scalar + 'static>(bytes: usize) -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        demand,
    } = super::super::super::setup::<P>()?;
    let prelude = prelude::<P>(1, &mut account.ledger)?;
    operation(&mut request, &prelude)?;
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    assert_eq!(constants(&heap), (2, bytes));
    let owner = candidate(owner, &mut account)?;
    let tile = publish(&mut heap, &mut account, owner)?;
    let mut request = super::super::super::next_copy(&mut heap, &mut account, tile.into_pin())?;
    let next_demand = request.demand;
    operation(&mut request, &prelude)?;
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    assert_eq!(constants(&heap), (2, bytes));
    let owner = candidate(owner, &mut account)?;
    drop(publish(&mut heap, &mut account, owner)?);
    heap.retire_mask(next_demand, &mut account.ledger)?;
    heap.retire_mask(demand, &mut account.ledger)?;
    super::super::super::retire_copy_layout(&mut heap, &mut account, demand.layout())?;
    for _ in 0..2 {
        assert!(matches!(
            heap.residence(&mut account, 100_000)?,
            crate::Residence::Pending { .. }
        ));
    }
    assert_eq!(constants(&heap), (2, bytes));
    drop(prelude);
    super::super::super::empty(&mut heap, &mut account)?;
    assert_eq!(constants(&heap), (0, 0));
    Ok(())
}

/// Count only the separate constant debit class, never infer expected bytes from its record.
fn constants(heap: &Heap) -> (usize, usize) {
    heap.loans
        .debits
        .iter()
        .flatten()
        .filter(|debit| {
            matches!(
                debit.kind,
                crate::loan::registry::AllocationKind::Constant(_)
            )
        })
        .fold((0, 0), |(count, bytes), debit| {
            (count + 1, bytes + debit.bytes)
        })
}

fn ready<P: Scalar>(result: ApplicationRequest<P>) -> TestResult<OwnedApplication<P>> {
    match result {
        ApplicationRequest::ApplicationReady(owner) => Ok(owner),
        other @ ApplicationRequest::ApplicationRefused(..) => Err(format!("{other:?}").into()),
    }
}

/// An empty debit table requires two owner visits and four destruction units: 6.
/// Its neighbour 5 refuses without a retaining edge, allocation or heap mutation.
/// ´test:unit:loan-affine-prelude-retention-work-boundary´
#[test]
fn loan_affine_prelude_retention_work_boundary() -> TestResult {
    retention_work::<f32>(152)?;
    retention_work::<f64>(208)
}

fn retention_work<P: Scalar + 'static>(bytes: usize) -> TestResult {
    use nrz_bindings::{Budget, Limits};
    let heap = Heap::new(crate::HeapLimits::default())?;
    let prelude = prelude::<P>(1, &mut Budget::new(Limits::default())?)?;
    for available in [5, 6] {
        let mut budget = Budget::new(Limits {
            work: available,
            ..Limits::default()
        })?;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result =
            crate::loan::application::affine::reserve(&heap, Some(&prelude.erase()), &mut budget);
        if available == 6 {
            let owners = result?;
            assert_eq!(owners.iter().flatten().count(), 2);
            assert_eq!(
                owners
                    .iter()
                    .flatten()
                    .map(|owner| owner.bytes)
                    .sum::<usize>(),
                bytes
            );
        } else {
            assert!(result.is_err());
        }
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        assert_eq!(constants(&heap), (0, 0));
        drop(injection);
    }
    Ok(())
}

/// A failed application preparation installs no numerical-owner debits and returns the input.
/// ´test:unit:loan-affine-prelude-admission-atomic´
#[test]
fn loan_affine_prelude_admission_atomic() -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = super::super::super::setup::<f64>()?;
    let prelude = prelude::<f64>(1, &mut account.ledger)?;
    operation(&mut request, &prelude)?;
    let ticket = request.input.ticket()?;
    crate::loan::fault::reset_reads();
    let injection = crate::loan::fault::Injection::after(0);
    let request = match heap.request_projected_affine(request, &mut account) {
        ApplicationRequest::ApplicationRefused(_, request) => request,
        ApplicationRequest::ApplicationReady(_) => {
            return Err("injected allocation admitted".into());
        }
    };
    assert_eq!(request.input.ticket()?, ticket);
    assert_eq!(constants(&heap), (0, 0));
    assert_eq!(crate::loan::fault::reads(), 0);
    drop(injection);
    let _owner = ready(heap.request_projected_affine(request, &mut account))?;
    assert_eq!(constants(&heap), (2, 208));
    Ok(())
}

/// A caller-held pi owner keeps its literal 96/128-byte debit after the prelude retires.
/// ´test:unit:loan-affine-caller-pi-retention´
#[test]
fn loan_affine_caller_pi_retention() -> TestResult {
    caller_pi::<f32>(96)?;
    caller_pi::<f64>(128)
}

fn caller_pi<P: Scalar + 'static>(pi_bytes: usize) -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        demand,
    } = super::super::super::setup::<P>()?;
    let context = nrz_scalar::ArithmeticContext::new()?;
    let circle = super::circle::<P>(&context, &mut account.ledger)?;
    let prelude = super::prelude_with_pi(1, &circle, &context, &mut account.ledger)?;
    operation(&mut request, &prelude)?;
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    let owner = candidate(owner, &mut account)?;
    drop(publish(&mut heap, &mut account, owner)?);
    heap.retire_mask(demand, &mut account.ledger)?;
    super::super::super::retire_copy_layout(&mut heap, &mut account, demand.layout())?;
    drop(prelude);
    for _ in 0..2 {
        assert!(matches!(
            heap.residence(&mut account, 100_000)?,
            crate::Residence::Pending { .. }
        ));
    }
    assert_eq!(constants(&heap), (1, pi_bytes));
    drop(circle);
    super::super::super::empty(&mut heap, &mut account)?;
    assert_eq!(constants(&heap), (0, 0));
    Ok(())
}

/// Three-channel host admission/publication leaves 15 rows with 8 vacancies. The first
/// affine admission reuses those vacancies and adds 19 debits: 15+19-8=26 rows.
/// Two scans, two owner visits and three destruction units cost 57 Work; 56 refuses.
/// The second admission's
/// eighteen debits use twenty-one identities and add only 56/80 constant bytes.
/// ´test:unit:loan-affine-shared-pi-admission´
#[test]
fn loan_affine_shared_pi_admission() -> TestResult {
    shared_pi::<f32>(56, 152)?;
    shared_pi::<f64>(80, 208)
}

fn shared_pi<P: Scalar + 'static>(coefficient_bytes: usize, initial_bytes: usize) -> TestResult {
    let Setup {
        mut heap,
        mut account,
        mut request,
        ..
    } = super::super::super::setup::<P>()?;
    let context = nrz_scalar::ArithmeticContext::new()?;
    let circle = super::circle::<P>(&context, &mut account.ledger)?;
    let first = super::prelude_with_pi(1, &circle, &context, &mut account.ledger)?;
    let second = super::prelude_with_pi(0, &circle, &context, &mut account.ledger)?;
    operation(&mut request, &first)?;
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    assert_eq!(constants(&heap), (2, initial_bytes));
    assert_eq!(heap.loans.debits.len(), 26);
    shared_work(&heap, &second, coefficient_bytes)?;
    let owner = candidate(owner, &mut account)?;
    let tile = publish(&mut heap, &mut account, owner)?;
    let mut request = super::super::super::next_copy(&mut heap, &mut account, tile.into_pin())?;
    operation(&mut request, &second)?;
    let owner = ready(heap.request_projected_affine(request, &mut account))?;
    assert_eq!(constants(&heap), (3, initial_bytes + coefficient_bytes));
    let owner = candidate(owner, &mut account)?;
    drop(publish(&mut heap, &mut account, owner)?);
    Ok(())
}

fn shared_work<P: Scalar + 'static>(
    heap: &Heap,
    prelude: &crate::AffinePrelude<P>,
    bytes: usize,
) -> TestResult {
    use nrz_bindings::{Budget, Limits};
    for available in [56, 57] {
        let mut budget = Budget::new(Limits {
            work: available,
            ..Limits::default()
        })?;
        crate::loan::fault::reset_reads();
        let injection = crate::loan::fault::Injection::after(0);
        let result =
            crate::loan::application::affine::reserve(heap, Some(&prelude.erase()), &mut budget);
        if available == 57 {
            let owners = result?;
            assert_eq!(owners.iter().flatten().count(), 1);
            assert_eq!(
                owners
                    .iter()
                    .flatten()
                    .map(|owner| owner.bytes)
                    .sum::<usize>(),
                bytes
            );
        } else {
            assert!(result.is_err());
        }
        assert_eq!(budget.usage().work, available);
        assert_eq!(
            [
                crate::loan::fault::reads(),
                crate::loan::fault::Injection::attempts()
            ],
            [0; 2]
        );
        drop(injection);
    }
    Ok(())
}
