// SPDX-License-Identifier: AGPL-3.0-only
//! Shared numerical prelude allocations join the common inventory only at atomic admission.

use crate::kernels::affine::NativeOwner;
use crate::loan::{
    LoanError, charge,
    registry::{AllocationKind, DebitOwner},
};
use nrz_bindings::Budget;

/// Count each immutable native allocation once, including independent callers still retaining it.
pub(in crate::loan) fn reserve(
    heap: &crate::Heap,
    prelude: Option<&crate::kernels::affine::Prelude>,
    budget: &mut Budget,
) -> Result<[Option<NativeOwner>; 2], LoanError> {
    let Some(prelude) = prelude else {
        return Ok([None, None]);
    };
    budget.with_work_phase(super::FAMILY, "affine prelude retention", |budget| {
        charge(budget, 2)?;
        let mut result = [None, None];
        for (index, owner) in prelude.owners().into_iter().enumerate() {
            let mut present = false;
            for debit in &heap.loans.debits {
                charge(budget, 1)?;
                if let Some(debit) = debit
                    && let AllocationKind::Constant(witness) = &debit.kind
                    && witness.ptr_eq(&owner.witness)
                {
                    if present
                        || debit.owner != DebitOwner::Constant(heap.scope)
                        || debit.bytes != owner.bytes
                    {
                        return Err(LoanError::WrongLoan);
                    }
                    present = true;
                }
            }
            if !present {
                charge(budget, owner.cleanup)?;
                result[index] = Some(owner);
            }
        }
        Ok(result)
    })
}
