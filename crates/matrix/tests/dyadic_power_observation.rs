// SPDX-License-Identifier: AGPL-3.0-only
//! Isolated subscriber control for the dyadic-power numerical leaf.
use nrz_scalar::{
    ArithmeticContext, ArithmeticFailure, DyadicPowerFailure, DyadicPowerLimits, DyadicPowerProgram,
};
use std::io::Write as _;

/// Fallible result for the isolated observation fixture.
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Capacities admitting the observed ordinary and long refusal schedules.
const LIMITS: DyadicPowerLimits = DyadicPowerLimits {
    exponent_bits: 63,
    root_steps: 63,
    product_steps: 124,
};

/// Collect span names, fields and exact Work events for numerical observation.
#[derive(Clone, Default)]
struct PowerObserver {
    /// Next identifier assigned to a visited span.
    next: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Span names keyed by their assigned identifiers.
    names: std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<u64, String>>>,
    /// Recorded span fields with their identifiers and names.
    fields: std::sync::Arc<std::sync::Mutex<Vec<(u64, String, String)>>>,
    /// Number of exact Work events seen by this subscriber.
    exact_events: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// Record debug field values for one visited span.
struct PowerFields {
    /// Identifier of the span being visited.
    id: u64,
    /// Shared ordered buffer of visited field values.
    fields: std::sync::Arc<std::sync::Mutex<Vec<(u64, String, String)>>>,
}

impl tracing::field::Visit for PowerFields {
    /// Save a debug field value with its span identifier.
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.lock().expect("field visitor lock").push((
            self.id,
            field.name().to_owned(),
            format!("{value:?}"),
        ));
    }
}

impl tracing::Subscriber for PowerObserver {
    /// Enable every span and event during this local observation.
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    /// Assign an identifier and record a new span's initial fields.
    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        use std::sync::atomic::Ordering as AtomicOrdering;
        let id = self.next.fetch_add(1, AtomicOrdering::Relaxed) + 1;
        self.names
            .lock()
            .expect("span names lock")
            .insert(id, span.metadata().name().to_owned());
        span.record(&mut PowerFields {
            id,
            fields: self.fields.clone(),
        });
        tracing::span::Id::from_u64(id)
    }
    /// Record updated fields for an existing span.
    fn record(&self, span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        values.record(&mut PowerFields {
            id: span.into_u64(),
            fields: self.fields.clone(),
        });
    }
    /// Ignore causal links because these assertions inspect span fields.
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    /// Count events on the exact Work tracing target.
    fn event(&self, event: &tracing::Event<'_>) {
        use std::sync::atomic::Ordering as AtomicOrdering;
        if event.metadata().target() == "nrz::work" {
            self.exact_events.fetch_add(1, AtomicOrdering::Relaxed);
        }
    }
    /// Ignore span entry because this test reads recorded fields.
    fn enter(&self, _: &tracing::span::Id) {}
    /// Ignore span exit because this test reads recorded fields.
    fn exit(&self, _: &tracing::span::Id) {}
}

/// Return the latest recorded value of one field on one span.
fn field<'a>(fields: &'a [(u64, String, String)], id: u64, name: &str) -> Option<&'a str> {
    fields
        .iter()
        .rev()
        .find(|(span, key, _)| *span == id && key == name)
        .map(|(_, _, value)| value.as_str())
}

/// A visiting subscriber sees one input and outcome per call, actual bounded
/// prefix counts, and no exact Work in the numerical leaf.
/// # Panics
/// Panics if a span omits its original point, result or completed stage counts.
/// ´claim:matrix:scalar-dyadic-power-observation´
/// ´test:integration:scalar-dyadic-power-observation´
#[expect(
    clippy::too_many_lines,
    reason = "one selected observer checks lowering, validation and kernel fields"
)]
#[test]
fn scalar_dyadic_power_observation() -> TestResult {
    use std::sync::atomic::Ordering as AtomicOrdering;
    let observer = PowerObserver::default();
    tracing::subscriber::with_default(observer.clone(), || -> TestResult {
        let context = ArithmeticContext::new()?;
        let ordinary = DyadicPowerProgram::new([f64::MIN_POSITIVE, 2.0], 3, 4, LIMITS, &context)?;
        let costly = DyadicPowerProgram::new(
            [f64::MIN_POSITIVE, 2.0],
            i64::MAX,
            1_u64 << 63,
            LIMITS,
            &context,
        )?;
        ordinary.evaluate(1.0, &context)?;
        ordinary.evaluate(0.5, &context)?;
        assert_eq!(
            costly.evaluate(2.0, &context).expect_err("refused prefix"),
            DyadicPowerFailure::TargetNotCertifiable {
                failure: ArithmeticFailure::Overflow
            }
        );
        Ok(())
    })?;
    let names = observer.names.lock().map_err(|_| "span names")?;
    let kernels: Vec<_> = names
        .iter()
        .filter(|(_, name)| name.as_str() == "nrz.kernel")
        .map(|(id, _)| *id)
        .collect();
    let lowerings: Vec<_> = names
        .iter()
        .filter(|(_, name)| name.as_str() == "nrz.lower")
        .map(|(id, _)| *id)
        .collect();
    let validations: Vec<_> = names
        .iter()
        .filter(|(_, name)| name.as_str() == "nrz.validation")
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(kernels.len(), 3);
    assert_eq!(lowerings.len(), 2);
    assert_eq!(validations.len(), 3);
    drop(names);
    let fields = observer.fields.lock().map_err(|_| "span fields")?;
    for (id, numerator, roots) in [
        (lowerings[0], "3", "2"),
        (lowerings[1], "9223372036854775807", "63"),
    ] {
        assert_eq!(field(&fields, id, "canonical_numerator"), Some(numerator));
        assert_eq!(field(&fields, id, "canonical_root_steps"), Some(roots));
        assert!(
            field(&fields, id, "result")
                .ok_or("lowering result")?
                .starts_with("Ok(")
        );
    }
    for (id, x) in validations.into_iter().zip(["1.0", "0.5", "2.0"]) {
        assert_eq!(field(&fields, id, "x"), Some(x));
        assert_eq!(field(&fields, id, "result"), Some("Ok(())"));
    }
    for (index, id) in kernels.into_iter().enumerate() {
        let expected_x = ["1.0", "0.5", "2.0"][index];
        assert_eq!(field(&fields, id, "x"), Some(expected_x));
        assert_eq!(
            fields
                .iter()
                .filter(|(span, name, _)| *span == id && name == "x")
                .count(),
            1
        );
        let outcome = field(&fields, id, "result").ok_or("kernel result")?;
        assert_eq!(
            fields
                .iter()
                .filter(|(span, name, _)| *span == id && name == "result")
                .count(),
            1
        );
        assert!(outcome.starts_with(if index == 2 { "Err(" } else { "Ok(" }));
        let mut counts = [0_usize; 5];
        for (slot, name) in counts.iter_mut().zip([
            "roots",
            "squares",
            "multiplies",
            "midpoint_steps",
            "displacement_steps",
        ]) {
            *slot = field(&fields, id, name).ok_or("stage count")?.parse()?;
        }
        match index {
            0 => assert_eq!(counts, [0, 0, 0, 0, 0]),
            1 => assert_eq!(counts, [2, 1, 1, 3, 2]),
            _ => assert_eq!(counts, [63, 58, 58, 0, 0]),
        }
        writeln!(
            std::io::stdout(),
            "DYADIC-OBSERVE x={expected_x} counts={counts:?} result={outcome}"
        )?;
    }
    assert_eq!(observer.exact_events.load(AtomicOrdering::Relaxed), 0);
    assert!(fields.iter().all(|(_, name, _)| !name.contains("owner")));
    drop(fields);
    Ok(())
}
