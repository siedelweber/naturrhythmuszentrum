// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded prepared-binary64 golden admission, shared by tooling and consumers.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

use nrz_cas::limits::Budget;
use nrz_cas::{Fault, Rational};
use serde_json::{Map, Value};
use thiserror::Error;

mod constant;
mod json;
mod preflight;
#[cfg(test)]
mod tests;

/// A structural refusal, separate from exact-arithmetic resource faults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    /// Invalid JSON or a duplicate object member.
    Json,
    /// An object has missing, extra or wrongly typed fields.
    Fields,
    /// The namespace is not the prepared-binary64 namespace.
    Namespace,
    /// The envelope or checker version is outside the admitted exact versions.
    Version,
    /// The owner, license, sources or labels are invalid.
    Envelope,
    /// A route kind is outside the closed pair.
    RouteKind,
    /// Route metadata or ordered references are invalid, or a width goal differs from its registry precision.
    Routes,
    /// Entry identifiers are empty or repeated.
    EntryId,
    /// The operation or expression is outside its closed row kind.
    Operation,
    /// A target or width goal does not match its row kind.
    Target,
    /// The expected-result form or refusal reason is unknown or row-incompatible.
    Expected,
    /// A power exponent is incompatible with its expected outcome.
    Exponent,
    /// A number is not a canonical reduced rational string.
    Rational,
    /// An input or golden interval has reversed endpoints.
    Ordering,
    /// Evidence is missing, repeated, stray, reordered or bound to another input, expression, context or judgment.
    Association,
    /// The two route intervals are disjoint.
    EmptyIntersection,
    /// The published interval differs from the exact intersection.
    Intersection,
    /// A published endpoint or judgment interval exceeds its width ceiling.
    Width,
    /// A promised exact result is not the required singleton.
    Exact,
    /// The byte ceiling was exceeded before parsing.
    Bytes,
    /// The entry ceiling was exceeded before JSON tree allocation.
    Entries,
    /// A decoded rational spelling exceeds its ceiling before JSON tree allocation.
    Literal,
    /// Target or width-goal precision exceeds the declared encoding ceiling.
    TargetBits,
    /// JSON nesting exceeds the declared ceiling or the hard maximum.
    Depth,
}

/// Golden admission never relabels arithmetic exhaustion as malformed data.
#[derive(Debug, Error)]
pub enum Error {
    /// A structurally invalid field, with its JSON location.
    #[error("golden {cause:?} at {path}")]
    Invalid {
        /// Closed structural or resource cause.
        cause: Cause,
        /// JSON field location.
        path: String,
    },
    /// The exact engine's original typed cause; the caller retains its ledger.
    #[error("golden arithmetic: {0}")]
    Arithmetic(#[from] Fault),
    /// Reading the bounded stream failed.
    #[error("golden input: {0}")]
    Io(#[from] std::io::Error),
}

/// Caller-visible parsing ceilings, in addition to the caller's CAS budget.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum input bytes, checked before allocating the JSON tree.
    pub bytes: usize,
    /// Maximum rows, checked before JSON tree allocation.
    pub entries: usize,
    /// Maximum UTF-8 bytes in each rational spelling.
    pub literal_bytes: usize,
    /// Maximum encoded target precision; not an attainable-target promise.
    pub target_bits: u64,
    /// Maximum JSON nesting, at most 64, checked before tree allocation.
    pub depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 1_048_576,
            entries: 256,
            literal_bytes: 4096,
            target_bits: 4096,
            depth: 64,
        }
    }
}

/// An admitted payload; no mutable access bypasses the checker.
#[derive(Debug)]
pub struct Payload(Value);

impl Payload {
    /// Borrow the checked JSON, retaining source and entry order.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.0
    }
}

/// Read and validate a complete bounded payload with cumulative exact Work.
///
/// The stream uses a fixed stack buffer and checks each extension before allocation.
/// An explicit nesting scan precedes JSON tree allocation. No route is executed here.
///
/// # Errors
/// Returns a typed structural refusal, I/O failure or the unchanged CAS fault.
pub fn read(input: impl Read, limits: Limits, budget: &mut Budget) -> Result<Payload, Error> {
    let value = decode(input, limits)?;
    validate(&value, limits, &mut |row, ids| {
        entry(row, ids, limits, budget)
    })?;
    Ok(Payload(value))
}

/// Read a bounded tree before choosing the arithmetic ledger for each entry.
fn decode(mut input: impl Read, limits: Limits) -> Result<Value, Error> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let count = input.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        let next = bytes
            .len()
            .checked_add(count)
            .ok_or_else(|| invalid(Cause::Bytes, "$"))?;
        ensure(next <= limits.bytes, Cause::Bytes, "$")?;
        bytes
            .try_reserve_exact(count)
            .map_err(|_| invalid(Cause::Bytes, "$"))?;
        bytes.extend_from_slice(&chunk[..count]);
    }
    json::parse(&bytes, limits, &mut json::Audit::default())
}

/// Bound structural nesting while ignoring brackets inside escaped JSON strings.
fn check_depth(bytes: &[u8], limit: usize) -> Result<(), Error> {
    ensure(limit <= 64, Cause::Depth, "$")?;
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'[' | b'{' => {
                    depth += 1;
                    ensure(depth <= limit, Cause::Depth, "$")?;
                }
                b']' | b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    Ok(())
}

/// Check every authored golden input when the optional input directory exists.
pub(crate) fn check_inputs(root: &Path) -> Result<(), crate::Error> {
    let directory = root.join("data/golden");
    let listing = match std::fs::read_dir(&directory) {
        Ok(listing) => listing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(crate::error::io("data/golden", error)),
    };
    let mut paths = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|error| crate::error::io("data/golden", error))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| crate::error::io("data/golden", error))?;
        if !kind.is_file() || path.extension().is_none_or(|extension| extension != "json") {
            return Err(crate::Error::Golden {
                path: path.display().to_string(),
                source: invalid(Cause::Fields, "golden input layout"),
            });
        }
        paths.push(path);
    }
    paths.sort();
    for path in paths {
        let display = path.display().to_string();
        let file = std::fs::File::open(&path).map_err(|error| crate::error::io(&display, error))?;
        let limits = Limits::default();
        let value = decode(file, limits).map_err(|source| crate::Error::Golden {
            path: display.clone(),
            source,
        })?;
        validate_profiled(&value, limits, |_| checker_limits()).map_err(|source| {
            crate::Error::Golden {
                path: display,
                source,
            }
        })?;
    }
    Ok(())
}

/// Per-entry policy: twice the largest measured row Work and rational spelling.
fn checker_limits() -> nrz_cas::limits::Limits {
    nrz_cas::limits::Limits {
        work: 7_889_442,
        literal_bytes: 618,
        ..nrz_cas::limits::Limits::default()
    }
}

/// Admit each entry on a fresh finite ledger; the bounded count limits total Work.
fn validate_profiled(
    value: &Value,
    limits: Limits,
    mut profile: impl FnMut(usize) -> nrz_cas::limits::Limits,
) -> Result<(), Error> {
    let count = Record::new(value, "$")?.array("entries")?.len();
    ensure(count <= limits.entries, Cause::Entries, "$.entries")?;
    let mut total_limits = checker_limits();
    total_limits.work = payload_work(count, total_limits.work)?;
    let mut total = Budget::new(total_limits)?;
    let mut index = 0;
    validate(value, limits, &mut |row, ids| {
        let mut budget = Budget::new(profile(index))?;
        index += 1;
        let result = entry(row, ids, limits, &mut budget);
        total.work(
            budget.usage().work,
            nrz_cas::Location::new("golden payload"),
        )?;
        result
    })
}

/// Checked product of the admitted count and the measured per-entry tariff.
fn payload_work(count: usize, per_entry: u64) -> Result<u64, Fault> {
    u64::try_from(count)
        .ok()
        .and_then(|count| count.checked_mul(per_entry))
        .ok_or_else(|| {
            Fault::capacity(
                nrz_cas::limits::BudgetKind::Work,
                nrz_cas::Location::new("golden payload"),
            )
        })
}

/// Validate the reserved envelope and the closed route registry before rows.
fn validate(
    value: &Value,
    limits: Limits,
    check_entry: &mut impl FnMut(&Record<'_>, &[&str]) -> Result<(), Error>,
) -> Result<(), Error> {
    let root = Record::new(value, "$")?;
    root.keys(&[
        "namespace",
        "version",
        "owner",
        "license",
        "sources",
        "labels",
        "routes",
        "entries",
    ])?;
    ensure(
        root.text("namespace")? == "org.naturrhythmuszentrum.test.golden.prepared-binary64",
        Cause::Namespace,
        "$.namespace",
    )?;
    ensure(
        root.get("version")? == &serde_json::json!([1, 1, 0]),
        Cause::Version,
        "$.version",
    )?;
    ensure(
        root.text("owner")? == "NRZ" && root.text("license")? == "CC0-1.0",
        Cause::Envelope,
        "$",
    )?;
    for field in ["sources", "labels"] {
        let mut seen = BTreeSet::new();
        for item in root.array(field)? {
            let text = item
                .as_str()
                .ok_or_else(|| invalid(Cause::Envelope, field))?;
            let valid = if field == "sources" {
                crate::envelope::normal_path(text)
            } else {
                crate::envelope::valid_label(text)
            };
            ensure(valid && seen.insert(text), Cause::Envelope, field)?;
        }
    }
    let routes = root.array("routes")?;
    ensure(routes.len() == 2, Cause::Routes, "$.routes")?;
    let mut ids = Vec::new();
    for (route, kind) in routes.iter().zip(["nrz-cas-enclosure", "mpmath-interval"]) {
        let record = Record::new(route, "$.routes")?;
        let precision = if kind == "nrz-cas-enclosure" {
            "precision_bits"
        } else {
            "precision_decimal_digits"
        };
        record.keys(&["id", "kind", "tool", "revision", precision, "argument"])?;
        ensure(
            record.text("kind")? == kind,
            Cause::RouteKind,
            "$.routes.kind",
        )?;
        for field in ["id", "tool", "revision", "argument"] {
            ensure(!record.text(field)?.trim().is_empty(), Cause::Routes, field)?;
        }
        ensure(
            record.get(precision)?.as_u64().is_some_and(|n| n > 0),
            Cause::Routes,
            precision,
        )?;
        let id = record.text("id")?;
        ensure(!ids.contains(&id), Cause::Routes, "$.routes.id")?;
        ids.push(id);
    }
    let entries = root.array("entries")?;
    ensure(entries.len() <= limits.entries, Cause::Entries, "$.entries")?;
    let mut names = BTreeSet::new();
    for (index, value) in entries.iter().enumerate() {
        let path = format!("$.entries[{index}]");
        let row = Record::new(value, &path)?;
        if row.object.contains_key("kind") {
            ensure(
                row.record("width_goal")?.get("bits")? == &routes[0]["precision_bits"],
                Cause::Routes,
                &path,
            )?;
        }
        let id = row.text("id")?;
        ensure(
            !id.is_empty() && names.insert(id.to_owned()),
            Cause::EntryId,
            &path,
        )?;
        check_entry(&row, &ids)?;
    }
    constant::associate_rows(entries)
}

/// Check one law's forms and endpoint-bound intersection evidence.
fn entry(row: &Record<'_>, ids: &[&str], limits: Limits, budget: &mut Budget) -> Result<(), Error> {
    if row.object.contains_key("kind") {
        return constant::entry(row, ids, limits, budget);
    }
    let operation = row.text("operation")?;
    ensure(
        matches!(operation, "exp" | "expm1" | "sigma" | "tanh" | "power"),
        Cause::Operation,
        row.path,
    )?;
    let enclosure = row.get("expected")?.as_str() == Some("enclosure");
    let mut keys = vec![
        "id",
        "operation",
        "input",
        "target",
        "expected",
        "routes",
        "evidence",
    ];
    if operation == "power" {
        keys.push("exponent");
    }
    if enclosure {
        keys.push("golden");
    }
    row.keys(&keys)?;
    let refusal = if enclosure {
        None
    } else {
        let expected = row.record("expected")?;
        expected.keys(&["refusal"])?;
        let reason = expected.text("refusal")?;
        ensure(
            matches!(
                reason,
                "outside-domain"
                    | "below-normal-operand"
                    | "non-dyadic-exponent"
                    | "target-not-certifiable"
            ),
            Cause::Expected,
            row.path,
        )?;
        ensure(
            reason != "non-dyadic-exponent" || operation == "power",
            Cause::Expected,
            row.path,
        )?;
        Some(reason)
    };
    let refs = row.array("routes")?;
    ensure(
        refs.len() == ids.len() && refs.iter().zip(ids).all(|(v, id)| v.as_str() == Some(id)),
        Cause::Routes,
        row.path,
    )?;
    let exponent = if operation == "power" {
        let exponent = rational(row.get("exponent")?, limits, budget, row.path)?;
        power_exponent(&exponent, row.text("exponent")?, refusal, budget, row.path)?;
        Some(exponent)
    } else {
        None
    };
    let target = row.record("target")?;
    if operation == "power" {
        target.keys(&["kind"])?;
        ensure(target.text("kind")? == "outward", Cause::Target, row.path)?;
    } else {
        target.keys(&["kind", "bits"])?;
        let kind = if matches!(operation, "expm1" | "tanh") {
            "relative"
        } else {
            "absolute"
        };
        ensure(target.text("kind")? == kind, Cause::Target, row.path)?;
        let bits = target
            .get("bits")?
            .as_u64()
            .ok_or_else(|| invalid(Cause::Target, row.path))?;
        ensure(bits <= limits.target_bits, Cause::TargetBits, row.path)?;
    }
    endpoint_evidence(
        row,
        ids,
        limits,
        budget,
        operation,
        exponent.as_ref(),
        enclosure,
    )
}

/// Classify canonical exponents exactly; finite CAS limits remain resource outcomes.
fn power_exponent(
    value: &Rational,
    text: &str,
    refusal: Option<&str>,
    budget: &mut Budget,
    path: &str,
) -> Result<(), Error> {
    let in_range = value.compare(&Rational::zero(), budget)? == Ordering::Greater
        && value.compare(&Rational::one(), budget)? != Ordering::Greater;
    if !in_range {
        return ensure(refusal == Some("outside-domain"), Cause::Exponent, path);
    }
    let dyadic = if let Some((_, denominator)) = text.split_once('/') {
        let denominator = Rational::parse(denominator, budget)?;
        let power = i64::try_from(denominator.component_bits().0 - 1).map_err(|_| {
            Fault::capacity(
                nrz_cas::limits::BudgetKind::IntegerExponent,
                nrz_cas::Location::new("golden exponent"),
            )
        })?;
        denominator == Rational::from_ratio(2, 1, budget)?.pow(power, budget)?
    } else {
        true
    };
    ensure(
        dyadic != (refusal == Some("non-dyadic-exponent")),
        Cause::Exponent,
        path,
    )
}

/// Check input bindings and the two independent intervals for each endpoint.
fn endpoint_evidence(
    row: &Record<'_>,
    ids: &[&str],
    limits: Limits,
    budget: &mut Budget,
    operation: &str,
    exponent: Option<&Rational>,
    enclosure: bool,
) -> Result<(), Error> {
    let input = row.record("input")?;
    let endpoints = if input.object.contains_key("point") {
        input.keys(&["point"])?;
        vec![("point", input.get("point")?)]
    } else {
        input.keys(&["box"])?;
        let pair = input.array("box")?;
        ensure(pair.len() == 2, Cause::Fields, row.path)?;
        vec![("lower_endpoint", &pair[0]), ("upper_endpoint", &pair[1])]
    };
    let numbers = endpoints
        .iter()
        .map(|(_, v)| rational(v, limits, budget, row.path))
        .collect::<Result<Vec<_>, _>>()?;
    if let [lower, upper] = numbers.as_slice() {
        ensure(
            lower.compare(upper, budget)? != Ordering::Greater,
            Cause::Ordering,
            row.path,
        )?;
    }
    let evidence = row.array("evidence")?;
    if !enclosure {
        return ensure(evidence.is_empty(), Cause::Association, row.path);
    }
    let golden = row.record("golden")?;
    golden.keys(&endpoints.iter().map(|(name, _)| *name).collect::<Vec<_>>())?;
    ensure(
        evidence.len() == endpoints.len(),
        Cause::Association,
        row.path,
    )?;
    for (((name, input_value), number), value) in endpoints.iter().zip(&numbers).zip(evidence) {
        let record = Record::new(value, row.path)?;
        record.keys(&["endpoint", "input", "routes"])?;
        ensure(
            record.text("endpoint")? == *name && record.get("input")? == *input_value,
            Cause::Association,
            row.path,
        )?;
        let routes = record.array("routes")?;
        ensure(routes.len() == ids.len(), Cause::Association, row.path)?;
        let mut intervals = Vec::new();
        for (route, id) in routes.iter().zip(ids) {
            let route = Record::new(route, row.path)?;
            route.keys(&["route", "interval"])?;
            ensure(route.text("route")? == *id, Cause::Association, row.path)?;
            intervals.push(interval(route.get("interval")?, limits, budget, row.path)?);
        }
        let first = &intervals[0];
        let second = &intervals[1];
        let lower = if first[0].compare(&second[0], budget)? == Ordering::Less {
            &second[0]
        } else {
            &first[0]
        };
        let upper = if first[1].compare(&second[1], budget)? == Ordering::Greater {
            &second[1]
        } else {
            &first[1]
        };
        ensure(
            lower.compare(upper, budget)? != Ordering::Greater,
            Cause::EmptyIntersection,
            row.path,
        )?;
        let published = interval(golden.get(name)?, limits, budget, row.path)?;
        ensure(
            &published[0] == lower && &published[1] == upper,
            Cause::Intersection,
            row.path,
        )?;
        let width = upper.sub(lower, budget)?;
        let ceiling = Rational::from_ratio(2, 1, budget)?.pow(-120, budget)?;
        ensure(
            width.compare(&ceiling, budget)? != Ordering::Greater,
            Cause::Width,
            row.path,
        )?;
        exact(operation, number, exponent, &published, budget, row.path)?;
    }
    Ok(())
}

/// Check promised elementary exact values without evaluating a transcendental.
fn exact(
    operation: &str,
    input: &Rational,
    exponent: Option<&Rational>,
    interval: &[Rational; 2],
    budget: &mut Budget,
    path: &str,
) -> Result<(), Error> {
    let one = Rational::one();
    let expected = match operation {
        "exp" if input.is_zero() => Some(one),
        "expm1" | "tanh" if input.is_zero() => Some(Rational::zero()),
        "sigma" if input.is_zero() => Some(Rational::from_ratio(1, 2, budget)?),
        "power" if input == &one => Some(one),
        "power" if exponent == Some(&one) => {
            return ensure(
                &interval[0] == input && &interval[1] == input,
                Cause::Exact,
                path,
            );
        }
        _ => None,
    };
    if let Some(expected) = expected {
        ensure(
            interval[0] == expected && interval[1] == expected,
            Cause::Exact,
            path,
        )?;
    }
    Ok(())
}

/// Decode and order one exact pair.
fn interval(
    value: &Value,
    limits: Limits,
    budget: &mut Budget,
    path: &str,
) -> Result<[Rational; 2], Error> {
    let pair = value
        .as_array()
        .ok_or_else(|| invalid(Cause::Fields, path))?;
    ensure(pair.len() == 2, Cause::Fields, path)?;
    let lower = rational(&pair[0], limits, budget, path)?;
    let upper = rational(&pair[1], limits, budget, path)?;
    ensure(
        lower.compare(&upper, budget)? != Ordering::Greater,
        Cause::Ordering,
        path,
    )?;
    Ok([lower, upper])
}

/// Admit only canonical integer or reduced fraction spelling, never decimals.
fn rational(
    value: &Value,
    limits: Limits,
    budget: &mut Budget,
    path: &str,
) -> Result<Rational, Error> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid(Cause::Rational, path))?;
    ensure(text.len() <= limits.literal_bytes, Cause::Literal, path)?;
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let mut pieces = unsigned.split('/');
    let numerator = pieces.next().unwrap_or_default();
    let denominator = pieces.next();
    let digits = |s: &str| {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'))
    };
    let valid = digits(numerator)
        && denominator.is_none_or(|d| digits(d) && d != "0")
        && pieces.next().is_none()
        && text != "-0";
    ensure(valid, Cause::Rational, path)?;
    let number = Rational::parse(text, budget)?;
    ensure(number.exact_string(budget)? == text, Cause::Rational, path)?;
    Ok(number)
}

/// Borrowed exact-key object validation.
struct Record<'a> {
    /// Borrowed object fields.
    object: &'a Map<String, Value>,
    /// Location retained in any structural refusal.
    path: &'a str,
}

impl<'a> Record<'a> {
    /// Require an object.
    fn new(value: &'a Value, path: &'a str) -> Result<Self, Error> {
        Ok(Self {
            object: value
                .as_object()
                .ok_or_else(|| invalid(Cause::Fields, path))?,
            path,
        })
    }
    /// Require precisely these members.
    fn keys(&self, keys: &[&str]) -> Result<(), Error> {
        ensure(
            self.object.len() == keys.len()
                && keys.iter().all(|key| self.object.contains_key(*key)),
            Cause::Fields,
            self.path,
        )
    }
    /// Borrow one required field.
    fn get(&self, key: &str) -> Result<&'a Value, Error> {
        self.object
            .get(key)
            .ok_or_else(|| invalid(Cause::Fields, self.path))
    }
    /// Borrow a text field.
    fn text(&self, key: &str) -> Result<&'a str, Error> {
        self.get(key)?
            .as_str()
            .ok_or_else(|| invalid(Cause::Fields, self.path))
    }
    /// Borrow an array field.
    fn array(&self, key: &str) -> Result<&'a [Value], Error> {
        self.get(key)?
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| invalid(Cause::Fields, self.path))
    }
    /// Borrow an object field.
    fn record(&self, key: &str) -> Result<Self, Error> {
        Self::new(self.get(key)?, self.path)
    }
}

/// Build a location-bearing refusal.
fn invalid(cause: Cause, path: &str) -> Error {
    Error::Invalid {
        cause,
        path: path.to_owned(),
    }
}

/// Require a named condition without losing its typed cause.
fn ensure(condition: bool, cause: Cause, path: &str) -> Result<(), Error> {
    if condition {
        Ok(())
    } else {
        Err(invalid(cause, path))
    }
}
