// SPDX-License-Identifier: AGPL-3.0-only
//! # Test index
//!
//! | Test | Area | Claim |
//! |------|------|-------|
//! | [`constant_golden_cas_route`] | docgen | Refined originals replay independently and match the retained judgment and route. |
//! | [`constant_golden_spacing`] | docgen | Integer-decoded adjacent endpoints prove the stored success and spacing refusal expectations. |
//! | [`constant_golden_series`] | docgen | An independent rational series bounds twice logarithm inside the published golden interval. |

use nrz_cas::domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr};
use nrz_cas::enclosure::{EnclosureCertificate, EnclosureGoal, EnclosureRequest, RefinementPolicy};
use nrz_cas::limits::{Budget, Limits};
use nrz_cas::syntax::{AtomicSort, Draft, Environment, ExpressionDraft, Field, Operator, Sort};
use nrz_cas::{Cas, FaultKind, Handle, InvalidCode, Rational, Reference};
use std::fmt::Write;

/// Fallible evidence construction within the private integration-test module.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Explicit finite resources for producing and replaying the golden evidence.
fn ledger() -> Result<Budget> {
    Ok(Budget::new(Limits {
        work: 2_000_000_000,
        literal_bytes: 4096,
        integer_exponent: 4096,
        arithmetic_steps: 50_000_000,
        ..Limits::default()
    })?)
}

struct Original {
    cas: Cas,
    request: EnclosureRequest,
    logarithm: Handle,
}

fn original() -> Result<Original> {
    let mut cas = Cas::new(ledger()?.limits().clone())?;
    let mut drafts = vec![
        Draft::Header(HeaderDraft::Core(Environment::default())),
        Draft::Region(RegionDraft {
            header: Reference::Local(0),
            scope: Vec::new(),
            ambient: AmbientDomain::Assignments,
            expression: RegionExpr::Ambient,
        }),
        Draft::Context(ContextDraft::Root {
            header: Reference::Local(0),
            admissible: Reference::Local(1),
            assumptions: Vec::new(),
        }),
    ];
    for (operator, operands) in [
        (
            Operator::Rational(Rational::from_ratio(2, 1, &mut ledger()?)?),
            vec![],
        ),
        (Operator::NaturalLog, vec![Reference::Local(3)]),
        (
            Operator::Mul,
            vec![Reference::Local(3), Reference::Local(4)],
        ),
    ] {
        drafts.push(Draft::Expression(ExpressionDraft {
            space: None,
            operator,
            sort: Sort::Value(AtomicSort::Scalar(Field::Real)),
            header: Reference::Local(0),
            scope: Vec::new(),
            binding: None,
            operands,
        }));
    }
    let handles = cas.admit(&drafts, &mut ledger()?)?;
    Ok(Original {
        request: EnclosureRequest {
            target: handles[5].clone(),
            context: handles[2].clone(),
            coordinates: Vec::new(),
            goal: EnclosureGoal::Width,
        },
        logarithm: handles[4].clone(),
        cas,
    })
}

/// Recompute without reading either retained golden interval or a native export.
///
/// # Panics
/// Panics only if CAS admits a substituted request or replay changes its interval.
fn reproduce(precision: u32) -> Result<serde_json::Value> {
    let policy = RefinementPolicy {
        version: 1,
        precision,
        grid_ceiling: 256,
        rounds: 1,
        reduction_ceiling: 4,
        term_ceiling: 128,
        checking_work: 0,
    };
    let mut source = original()?;
    let report = source
        .cas
        .enclose(&source.request, policy, &mut ledger()?, &mut |_| false)?;
    let checked = report
        .checked
        .ok_or_else(|| format!("no enclosure: {:?}", report.outcome))?;
    assert_eq!(checked.request().target, source.request.target);
    assert_eq!(checked.request().context, source.request.context);
    let bytes = checked.certificate().to_bytes(&mut ledger()?)?;
    let certificate = EnclosureCertificate::from_bytes(&bytes, &mut ledger()?)?;
    let mut fresh = original()?;
    let replay = fresh.cas.check_enclosure(
        &fresh.request,
        policy,
        &certificate,
        &mut ledger()?,
        &mut |_| false,
    )?;
    assert_eq!(
        replay.checked.ok_or("replay")?.interval(),
        checked.interval()
    );
    let mut substituted = fresh.request.clone();
    substituted.target = fresh.logarithm;
    assert_eq!(
        fresh
            .cas
            .check_enclosure(
                &substituted,
                policy,
                &certificate,
                &mut ledger()?,
                &mut |_| false
            )
            .expect_err("substituted identity")
            .kind,
        FaultKind::Invalid(InvalidCode::EndpointMismatch)
    );
    // Equal-spelled originals in another store do not substitute for a live handle.
    substituted = fresh.request.clone();
    substituted.target = source.request.target.clone();
    assert_eq!(
        fresh
            .cas
            .check_enclosure(
                &substituted,
                policy,
                &certificate,
                &mut ledger()?,
                &mut |_| false
            )
            .expect_err("substituted identity")
            .kind,
        FaultKind::Invalid(InvalidCode::ForeignHandle)
    );
    substituted = fresh.request.clone();
    substituted.context = source.request.context.clone();
    assert_eq!(
        fresh
            .cas
            .check_enclosure(
                &substituted,
                policy,
                &certificate,
                &mut ledger()?,
                &mut |_| false
            )
            .expect_err("substituted identity")
            .kind,
        FaultKind::Invalid(InvalidCode::ForeignHandle)
    );
    let mut scope = String::new();
    for byte in &certificate.scope {
        write!(scope, "{byte:02x}")?;
    }
    Ok(serde_json::json!({
        "interval": [checked.interval().lower().exact_string(&mut ledger()?)?, checked.interval().upper().exact_string(&mut ledger()?)?],
        "checker_version": certificate.version,
        "precision_bits": precision,
        "schema": checked.formal_schema().name,
        "scope_hex": scope,
    }))
}

/// Refined originals replay independently and match the retained judgment and route.
/// # Panics
/// Panics only if replay, identity rejection, refinement or retained evidence changes.
/// ´claim:docgen:constant-golden-cas-route´
/// ´test:integration:constant-golden-cas-route´
#[test]
fn constant_golden_cas_route() -> Result<()> {
    let coarse = reproduce(128)?;
    let fine = reproduce(160)?;
    let mut b = ledger()?;
    let parse = |v: &serde_json::Value, b: &mut nrz_cas::limits::Budget| {
        nrz_cas::Rational::parse(v.as_str().unwrap_or("invalid"), b)
    };
    assert_ne!(
        parse(&fine["interval"][0], &mut b)?
            .compare(&parse(&coarse["interval"][0], &mut b)?, &mut b)?,
        std::cmp::Ordering::Less
    );
    assert_ne!(
        parse(&fine["interval"][1], &mut b)?
            .compare(&parse(&coarse["interval"][1], &mut b)?, &mut b)?,
        std::cmp::Ordering::Greater
    );
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/golden/exact-constant.json");
    let checked = nrz_docgen::golden::read(
        std::fs::File::open(path)?,
        nrz_docgen::golden::Limits::default(),
        &mut b,
    )?;
    let rows = checked.value()["entries"].as_array().ok_or("entries")?;
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row["width_goal"]["bits"], fine["precision_bits"]);
        for field in ["interval", "checker_version", "scope_hex", "schema"] {
            assert_eq!(row["judgment"][field], fine[field], "judgment {field}");
        }
        assert_eq!(
            row["evidence"][0]["routes"][0]["interval"],
            fine["interval"]
        );
    }
    Ok(())
}

/// Decode a positive normal binary64 pattern with integer and rational arithmetic only.
fn binary64(bits: u64, budget: &mut Budget) -> Result<Rational> {
    let significand = (bits & ((1_u64 << 52) - 1)) + (1_u64 << 52);
    let exponent = i64::try_from((bits >> 52) & 2047)? - 1075;
    Ok(
        Rational::from_ratio(i64::try_from(significand)?, 1, budget)?.mul(
            &Rational::from_ratio(2, 1, budget)?.pow(exponent, budget)?,
            budget,
        )?,
    )
}

/// Integer-decoded adjacent endpoints prove the stored success and spacing refusal expectations.
/// # Panics
/// Panics only if row identities, endpoint placement or declared mathematical outcomes change.
/// ´claim:docgen:constant-golden-spacing´
/// ´test:integration:constant-golden-spacing´
#[test]
fn constant_golden_spacing() -> Result<()> {
    let mut b = ledger()?;
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/golden/exact-constant.json"))?;
    let rows = value["entries"].as_array().ok_or("entries")?;
    assert_eq!(rows.len(), 2);
    let patterns = [0x3ff6_2e42_fefa_39ef_u64, 0x3ff6_2e42_fefa_39f0];
    assert_eq!(patterns[1], patterns[0] + 1);
    for bits in patterns {
        assert_eq!(bits >> 52, 1023, "positive values in [1,2)");
    }
    let down = binary64(patterns[0], &mut b)?;
    let up = binary64(patterns[1], &mut b)?;
    let two = Rational::from_ratio(2, 1, &mut b)?;
    assert_eq!(up.sub(&down, &mut b)?, two.pow(-52, &mut b)?);
    for (id, bits, expected) in [
        ("twice-log-two-absolute", 40, serde_json::json!("enclosure")),
        (
            "twice-log-two-spacing",
            60,
            serde_json::json!({"refusal":"target-not-certifiable"}),
        ),
    ] {
        let row = rows.iter().find(|row| row["id"] == id).ok_or("row")?;
        assert_eq!(
            row["target"],
            serde_json::json!({"kind":"absolute", "bits":bits})
        );
        assert_eq!(row["expected"], expected);
        spacing_row(row, &down, &up, &mut b)?;
    }
    Ok(())
}

/// Both endpoint budgets and the obstruction hold throughout each whole golden interval.
/// # Panics
/// Panics only if the interval leaves its strict margins or changes a target conclusion.
fn spacing_row(
    row: &serde_json::Value,
    down: &Rational,
    up: &Rational,
    b: &mut Budget,
) -> Result<()> {
    use std::cmp::Ordering::{Greater, Less};
    let two = Rational::from_ratio(2, 1, b)?;
    let margin = two.pow(-128, b)?;
    let success = two.pow(-40, b)?;
    let floor = two.pow(-53, b)?;
    let lo = Rational::parse(row["golden"]["point"][0].as_str().ok_or("lower")?, b)?;
    let hi = Rational::parse(row["golden"]["point"][1].as_str().ok_or("upper")?, b)?;
    assert_ne!(lo.compare(&hi, b)?, Greater);
    assert_ne!(hi.sub(&lo, b)?.compare(&two.pow(-120, b)?, b)?, Greater);
    assert_eq!(lo.sub(down, b)?.compare(&margin, b)?, Greater);
    assert_eq!(up.sub(&hi, b)?.compare(&margin, b)?, Greater);
    assert_ne!(hi.sub(down, b)?.compare(&success, b)?, Greater);
    assert_ne!(up.sub(&lo, b)?.compare(&success, b)?, Greater);
    // At least one closest outward endpoint already exceeds 2^-53 for the
    // whole golden interval. Smaller allowances for every p >= 53 cannot fit.
    assert!(
        lo.sub(down, b)?.compare(&floor, b)? == Greater
            || up.sub(&hi, b)?.compare(&floor, b)? == Greater
    );
    assert_eq!(down.compare(&lo, b)?, Less);
    assert_eq!(hi.compare(up, b)?, Less);
    Ok(())
}

/// Compare independently derived bounds with a proposed published interval.
fn series_inside(bounds: &[Rational; 2], golden: &[Rational; 2], b: &mut Budget) -> Result<bool> {
    use std::cmp::Ordering::{Greater, Less};
    Ok(bounds[0].compare(&golden[0], b)? != Less && bounds[1].compare(&golden[1], b)? != Greater)
}

/// An independent rational series bounds twice logarithm inside the published golden interval.
/// # Panics
/// Panics only if the bounds fail their longer-sum or golden comparisons.
/// ´claim:docgen:constant-golden-series´
/// ´test:integration:constant-golden-series´
#[test]
fn constant_golden_series() -> Result<()> {
    // No enclosure construction, replay, schedule or numerical helper is used.
    let mut b = Budget::new(Limits {
        work: 2_000_000_000,
        literal_bytes: 4096,
        component_bits: 4096,
        intermediate_bits: 8192,
        integer_exponent: 4096,
        arithmetic_steps: 50_000_000,
        ..Limits::default()
    })?;
    let two = Rational::from_ratio(2, 1, &mut b)?;
    let mut sum = Rational::from_ratio(0, 1, &mut b)?;
    let terms = 520;
    for k in 1..=terms {
        let term = Rational::from_ratio(1, k, &mut b)?
            .mul(&two.pow(-i64::try_from(k)?, &mut b)?, &mut b)?;
        sum = sum.add(&term, &mut b)?;
    }
    // For k > N, 1/k <= 1/(N+1); sum_{k>N} 2^-k = 2^-N.
    // Hence the omitted positive tail is at most 1/((N+1)*2^N).
    let tail = Rational::from_ratio(1, terms + 1, &mut b)?
        .mul(&two.pow(-i64::try_from(terms)?, &mut b)?, &mut b)?;
    let bounds = [
        two.mul(&sum, &mut b)?,
        two.mul(&sum.add(&tail, &mut b)?, &mut b)?,
    ];
    // Sixteen more terms make the remaining tail less than 2^-16 of the
    // 520-term tail bound. Recompute from zero to detect a shortened first sum.
    let mut longer = Rational::from_ratio(0, 1, &mut b)?;
    for k in 1..=536 {
        let term = Rational::from_ratio(1, k, &mut b)?
            .mul(&two.pow(-i64::try_from(k)?, &mut b)?, &mut b)?;
        longer = longer.add(&term, &mut b)?;
    }
    let longer = two.mul(&longer, &mut b)?;
    assert_eq!(
        bounds[0].compare(&longer, &mut b)?,
        std::cmp::Ordering::Less,
        "series longer-sum lower bound"
    );
    assert_eq!(
        bounds[1].compare(&longer, &mut b)?,
        std::cmp::Ordering::Greater,
        "series longer-sum upper bound"
    );
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/golden/exact-constant.json"))?;
    for row in value["entries"].as_array().ok_or("entries")? {
        let golden = [
            Rational::parse(row["golden"]["point"][0].as_str().ok_or("lower")?, &mut b)?,
            Rational::parse(row["golden"]["point"][1].as_str().ok_or("upper")?, &mut b)?,
        ];
        assert!(
            series_inside(&bounds, &golden, &mut b)?,
            "series containment"
        );
        // Each negative fixture changes only the endpoint tested by its conjunct.
        assert!(
            !series_inside(&bounds, &[bounds[1].clone(), golden[1].clone()], &mut b)?,
            "series lower comparison"
        );
        assert!(
            !series_inside(&bounds, &[golden[0].clone(), bounds[0].clone()], &mut b)?,
            "series upper comparison"
        );
    }
    Ok(())
}
