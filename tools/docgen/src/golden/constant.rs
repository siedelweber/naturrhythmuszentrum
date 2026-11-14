// SPDX-License-Identifier: AGPL-3.0-only
//! Closed expression provenance and exact golden evidence, never export authority.

use super::{Cause, Error, Limits, Record, ensure, interval, invalid};
use nrz_cas::Rational;
use nrz_cas::limits::Budget;
use std::cmp::Ordering;

/// Admit the sole expression row without running either numerical route.
pub(super) fn entry(
    row: &Record<'_>,
    ids: &[&str],
    limits: Limits,
    budget: &mut Budget,
) -> Result<(), Error> {
    row.keys(&[
        "id",
        "kind",
        "expression",
        "context",
        "judgment",
        "width_goal",
        "target",
        "expected",
        "routes",
        "evidence",
        "golden",
    ])?;
    ensure(
        row.text("kind")? == "expression-constant",
        Cause::Operation,
        row.path,
    )?;
    let expression = row.record("expression")?;
    expression.keys(&["id", "syntax"])?;
    ensure(
        !expression.text("id")?.is_empty() && expression.text("syntax")? == "Mul(2, Log(2))",
        Cause::Operation,
        row.path,
    )?;
    let context = row.record("context")?;
    context.keys(&["id", "domain", "declarations", "assumptions"])?;
    ensure(
        !context.text("id")?.is_empty()
            && context.text("domain")? == "real"
            && context.array("declarations")?.is_empty()
            && context.array("assumptions")?.is_empty(),
        Cause::Association,
        row.path,
    )?;
    let judgment = row.record("judgment")?;
    judgment.keys(&[
        "expression",
        "context",
        "schema",
        "checker_version",
        "scope_hex",
        "interval",
    ])?;
    association(&judgment, &expression, &context)?;
    ensure(
        judgment.text("schema")? == "enclosure.log-atanh",
        Cause::Association,
        row.path,
    )?;
    let scope = judgment.text("scope_hex")?;
    ensure(
        !scope.is_empty()
            && scope.len().is_multiple_of(2)
            && scope
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        Cause::Association,
        row.path,
    )?;
    ensure(
        judgment.get("checker_version")?.as_u64() == Some(1),
        Cause::Version,
        row.path,
    )?;
    let goal = row.record("width_goal")?;
    goal.keys(&["kind", "bits"])?;
    ensure(goal.text("kind")? == "width", Cause::Target, row.path)?;
    let precision = bits(&goal, limits)?;
    let target = row.record("target")?;
    target.keys(&["kind", "bits"])?;
    ensure(target.text("kind")? == "absolute", Cause::Target, row.path)?;
    let _ = bits(&target, limits)?;
    if row.get("expected")?.as_str() != Some("enclosure") {
        let expected = row.record("expected")?;
        expected.keys(&["refusal"])?;
        ensure(
            expected.text("refusal")? == "target-not-certifiable",
            Cause::Expected,
            row.path,
        )?;
    }
    evidence(
        row,
        [&expression, &context, &judgment],
        precision,
        ids,
        limits,
        budget,
    )
}

/// Check route association, the judgment width and the exact golden intersection.
fn evidence(
    row: &Record<'_>,
    [expression, context, judgment]: [&Record<'_>; 3],
    precision: u64,
    ids: &[&str],
    limits: Limits,
    budget: &mut Budget,
) -> Result<(), Error> {
    let refs = row.array("routes")?;
    ensure(
        refs.len() == ids.len() && refs.iter().zip(ids).all(|(v, id)| v.as_str() == Some(id)),
        Cause::Routes,
        row.path,
    )?;
    let evidence = row.array("evidence")?;
    ensure(evidence.len() == 1, Cause::Association, row.path)?;
    let evidence = Record::new(&evidence[0], row.path)?;
    evidence.keys(&["expression", "context", "routes"])?;
    association(&evidence, expression, context)?;
    let routes = evidence.array("routes")?;
    ensure(routes.len() == ids.len(), Cause::Association, row.path)?;
    let mut pairs = Vec::new();
    for (value, id) in routes.iter().zip(ids) {
        let route = Record::new(value, row.path)?;
        route.keys(&["route", "interval"])?;
        ensure(route.text("route")? == *id, Cause::Association, row.path)?;
        pairs.push(interval(route.get("interval")?, limits, budget, row.path)?);
    }
    let exact = interval(judgment.get("interval")?, limits, budget, row.path)?;
    ensure(exact == pairs[0], Cause::Association, row.path)?;
    let two = Rational::from_ratio(2, 1, budget)?;
    let width = exact[1].sub(&exact[0], budget)?;
    let goal = two.pow(
        -i64::try_from(precision).map_err(|_| invalid(Cause::TargetBits, row.path))?,
        budget,
    )?;
    ensure(
        width.compare(&goal, budget)? != Ordering::Greater,
        Cause::Width,
        row.path,
    )?;
    let lower = if pairs[0][0].compare(&pairs[1][0], budget)? == Ordering::Less {
        &pairs[1][0]
    } else {
        &pairs[0][0]
    };
    let upper = if pairs[0][1].compare(&pairs[1][1], budget)? == Ordering::Greater {
        &pairs[1][1]
    } else {
        &pairs[0][1]
    };
    ensure(
        lower.compare(upper, budget)? != Ordering::Greater,
        Cause::EmptyIntersection,
        row.path,
    )?;
    let golden = row.record("golden")?;
    golden.keys(&["point"])?;
    let published = interval(golden.get("point")?, limits, budget, row.path)?;
    ensure(
        &published[0] == lower && &published[1] == upper,
        Cause::Intersection,
        row.path,
    )?;
    ensure(
        upper
            .sub(lower, budget)?
            .compare(&two.pow(-120, budget)?, budget)?
            != Ordering::Greater,
        Cause::Width,
        row.path,
    )
}

/// Dataset names pair the evidence with its original recipe, not live graph handles.
fn association(
    record: &Record<'_>,
    expression: &Record<'_>,
    context: &Record<'_>,
) -> Result<(), Error> {
    ensure(
        record.text("expression")? == expression.text("id")?
            && record.text("context")? == context.text("id")?,
        Cause::Association,
        record.path,
    )
}

/// Width and target precisions share the existing finite encoding ceiling.
fn bits(record: &Record<'_>, limits: Limits) -> Result<u64, Error> {
    let bits = record
        .get("bits")?
        .as_u64()
        .ok_or_else(|| invalid(Cause::Target, record.path))?;
    ensure(bits <= limits.target_bits, Cause::TargetBits, record.path)?;
    Ok(bits)
}

/// Repeated original identities retain one complete binding after row admission.
pub(super) fn associate_rows(entries: &[serde_json::Value]) -> Result<(), Error> {
    let mut bindings = std::collections::BTreeMap::new();
    for value in entries.iter().filter(|value| value.get("kind").is_some()) {
        let row = Record::new(value, "$.entries")?;
        let expression = row.record("expression")?;
        let id = expression.text("id")?;
        let binding = ["expression", "context", "judgment", "evidence", "golden"]
            .map(|field| row.get(field))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(previous) = bindings.insert(id, binding.clone()) {
            ensure(previous == binding, Cause::Association, row.path)?;
        }
    }
    Ok(())
}
