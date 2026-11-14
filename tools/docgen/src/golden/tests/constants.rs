// SPDX-License-Identifier: AGPL-3.0-only
//! Version isolation, original associations and preallocation for constant golden rows.

use super::{Cause, Error, Limits, Value, budget, fixture, json, read, rejects};

/// The authored constant payload is the positive control for every mutation.
/// # Panics
/// Panics only if the committed golden input is malformed JSON.
fn constants() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../data/golden/exact-constant.json"
    ))
    .expect("constant golden JSON")
}

/// Both row forms admit only the current exact schema version.
/// # Panics
/// Panics only if a current payload refuses or an obsolete version admits.
/// ´claim:docgen:constant-golden-versions´
/// ´test:unit:constant-golden-versions´
#[test]
fn constant_golden_versions() {
    for value in [fixture(), constants()] {
        admits(&value);
        for version in [
            json!([1, 0, 0]),
            json!([1, 0, 1]),
            json!([1, 1, 1]),
            json!([2, 1, 0]),
        ] {
            let mut obsolete = value.clone();
            obsolete["version"] = version;
            rejects(&obsolete, Cause::Version);
        }
    }
}

/// A single valid row isolates its checks from the repeated-identity check.
/// # Panics
/// Panics only if the committed fixture ceases to have an entries array.
fn single() -> Value {
    let mut value = constants();
    value["entries"]
        .as_array_mut()
        .expect("entries")
        .truncate(1);
    value
}

/// Check a positive fixture through the public reader before mutating one premise.
/// # Panics
/// Panics only if fixture encoding or admission fails.
fn admits(value: &Value) {
    let bytes = serde_json::to_vec(value).expect("encoding");
    let mut limits = budget(100_000_000).limits().clone();
    // The positive encoding-boundary fixture must reach its exact width goal.
    limits.integer_exponent = 4096;
    limits.component_bits = 4097;
    let mut ledger = nrz_cas::limits::Budget::new(limits).expect("fixture limits");
    let result = read(bytes.as_slice(), Limits::default(), &mut ledger);
    assert!(result.is_ok(), "positive fixture: {result:?}");
}

/// Every new record rejects an otherwise harmless additional member.
/// # Panics
/// Panics only if an object path changes or an extra member loses its Fields cause.
/// ´claim:docgen:constant-golden-unknown-members´
/// ´test:unit:constant-golden-unknown-members´
#[test]
fn constant_golden_unknown_members() {
    for path in [
        "",
        "/expression",
        "/context",
        "/judgment",
        "/width_goal",
        "/target",
        "/expected",
        "/evidence/0",
        "/evidence/0/routes/0",
        "/golden",
    ] {
        let mut value = constants();
        value["entries"].as_array_mut().expect("entries").remove(0);
        let row = &mut value["entries"][0];
        row.pointer_mut(path)
            .expect("record path")
            .as_object_mut()
            .expect("record")
            .insert("unexpected".to_owned(), json!("extra"));
        rejects(&value, Cause::Fields);
    }
}

/// Closed values, encodings and list cardinalities each have a typed negative control.
/// # Panics
/// Panics only if a mutation loses its specific refusal, including a short route list.
/// ´claim:docgen:constant-golden-closed-fields´
/// ´test:unit:constant-golden-closed-fields´
#[test]
fn constant_golden_closed_fields() {
    for (path, replacement, cause) in [
        ("/kind", json!("another-expression"), Cause::Operation),
        ("/kind", json!(""), Cause::Operation),
        ("/context/domain", json!("complex"), Cause::Association),
        ("/context/declarations", json!(["x"]), Cause::Association),
        ("/judgment/scope_hex", json!(""), Cause::Association),
        ("/judgment/scope_hex", json!("0"), Cause::Association),
        ("/judgment/scope_hex", json!("AB"), Cause::Association),
        ("/judgment/scope_hex", json!("zz"), Cause::Association),
        ("/target/bits", json!("40"), Cause::Target),
        ("/target/bits", json!(-1), Cause::Target),
        ("/width_goal/kind", json!("relative"), Cause::Target),
        ("/width_goal/bits", json!(159), Cause::Routes),
    ] {
        let mut value = single();
        *value["entries"][0].pointer_mut(path).expect("field path") = replacement;
        rejects(&value, cause);
    }
    let mut value = single();
    value["entries"][0]["routes"]
        .as_array_mut()
        .expect("refs")
        .swap(0, 1);
    rejects(&value, Cause::Routes);
    // Keep the ordered prefix valid while changing only the reference count.
    for extra in [false, true] {
        let mut value = single();
        let refs = value["entries"][0]["routes"].as_array_mut().expect("refs");
        if extra {
            refs.push(refs[0].clone());
        } else {
            refs.pop().expect("reference");
        }
        rejects(&value, Cause::Routes);
    }
    cardinalities();
}

/// Short and long lists must return the association cause, never panic or truncate.
/// # Panics
/// Panics only if a fixture list disappears or a cardinality mutation loses its cause.
fn cardinalities() {
    for field in ["evidence", "routes"] {
        let mut value = single();
        let list = if field == "evidence" {
            &mut value["entries"][0]["evidence"]
        } else {
            &mut value["entries"][0]["evidence"][0]["routes"]
        };
        let list = list.as_array_mut().expect("list");
        let removed = list.pop().expect("member");
        rejects(&value, Cause::Association);
        let mut value = single();
        let list = if field == "evidence" {
            &mut value["entries"][0]["evidence"]
        } else {
            &mut value["entries"][0]["evidence"][0]["routes"]
        };
        list.as_array_mut().expect("list").push(removed);
        rejects(&value, Cause::Association);
    }
}

/// Empty identities are isolated from the separate association equality checks.
/// # Panics
/// Panics only if an empty but consistently referenced identity admits.
/// ´claim:docgen:constant-golden-empty-identities´
/// ´test:unit:constant-golden-empty-identities´
#[test]
fn constant_golden_empty_identities() {
    for field in ["expression", "context"] {
        let mut value = single();
        value["entries"][0][field]["id"] = json!("");
        value["entries"][0]["judgment"][field] = json!("");
        value["entries"][0]["evidence"][0][field] = json!("");
        rejects(
            &value,
            if field == "expression" {
                Cause::Operation
            } else {
                Cause::Association
            },
        );
    }
}

/// Synthetic exact intervals isolate reader geometry from the numerical provenance tests.
/// # Panics
/// Panics only if finite rational fixture construction fails.
fn geometry(bits: u64, bounds: [[i64; 2]; 3], scale: i64) -> Value {
    let mut value = single();
    value["entries"]
        .as_array_mut()
        .expect("entries")
        .truncate(1);
    value["routes"][0]["precision_bits"] = json!(bits);
    value["entries"][0]["width_goal"]["bits"] = json!(bits);
    let mut b = budget(100_000_000);
    let quantum = nrz_cas::Rational::from_ratio(2, 1, &mut b)
        .expect("two")
        .pow(-scale, &mut b)
        .expect("quantum");
    for (index, pair) in bounds.into_iter().enumerate() {
        let pair = pair.map(|n| {
            nrz_cas::Rational::from_ratio(n, 1, &mut b)
                .expect("integer")
                .mul(&quantum, &mut b)
                .expect("scaled")
                .exact_string(&mut b)
                .expect("spelling")
        });
        if index < 2 {
            value["entries"][0]["evidence"][0]["routes"][index]["interval"] = json!(pair);
        } else {
            value["entries"][0]["golden"]["point"] = json!(pair);
        }
    }
    value["entries"][0]["judgment"]["interval"] =
        value["entries"][0]["evidence"][0]["routes"][0]["interval"].clone();
    value
}

/// Exact width boundaries and non-nested intersections distinguish every geometric check.
/// # Panics
/// Panics only if a width boundary, disjoint pair or proper intersection changes admission.
/// ´claim:docgen:constant-golden-geometry´
/// ´test:unit:constant-golden-geometry´
#[test]
fn constant_golden_geometry() {
    // Judgment width equals its goal; a strict inequality must not refuse it.
    admits(&geometry(160, [[0, 1], [0, 1], [0, 1]], 160));
    // The intersection differs from both routes, killing a second-route shortcut.
    admits(&geometry(159, [[0, 2], [1, 3], [1, 2]], 160));
    let mut disjoint = geometry(160, [[0, 1], [0, 1], [0, 1]], 160);
    admits(&disjoint);
    disjoint["entries"][0]["evidence"][0]["routes"][1]["interval"] = json!(["1", "1"]);
    rejects(&disjoint, Cause::EmptyIntersection);
    // Keep registry precision fixed while only the consistently published width grows.
    admits(&geometry(119, [[0, 1], [0, 1], [0, 1]], 120));
    rejects(&geometry(119, [[0, 2], [0, 2], [0, 2]], 120), Cause::Width);
    let boundary = geometry(4096, [[0, 0], [0, 0], [0, 0]], 0);
    admits(&boundary);
    let mut beyond = boundary;
    beyond["routes"][0]["precision_bits"] = json!(4097);
    beyond["entries"][0]["width_goal"]["bits"] = json!(4097);
    rejects(&beyond, Cause::TargetBits);
}

/// Isolated mutations cannot substitute expression, context, judgment or route evidence.
/// # Panics
/// Panics only if a fixture path disappears or a mutation loses its typed refusal.
/// ´claim:docgen:constant-golden-association-controls´
/// ´test:unit:constant-golden-association-controls´
#[test]
fn constant_golden_association_controls() {
    for (path, replacement, cause) in [
        ("/expression/syntax", json!("Log(2)"), Cause::Operation),
        ("/expression/id", json!("another-node"), Cause::Association),
        ("/context/id", json!("another-context"), Cause::Association),
        ("/context/assumptions", json!(["extra"]), Cause::Association),
        (
            "/judgment/expression",
            json!("another-node"),
            Cause::Association,
        ),
        (
            "/judgment/context",
            json!("another-context"),
            Cause::Association,
        ),
        (
            "/evidence/0/expression",
            json!("another-node"),
            Cause::Association,
        ),
        ("/judgment/checker_version", json!(2), Cause::Version),
        (
            "/judgment/schema",
            json!("enclosure.exp-taylor"),
            Cause::Association,
        ),
        ("/judgment/scope_hex", json!("GG"), Cause::Association),
        ("/judgment/interval/0", json!("0"), Cause::Association),
        (
            "/evidence/0/context",
            json!("another-context"),
            Cause::Association,
        ),
        (
            "/evidence/0/routes/0/route",
            json!("mpmath-iv"),
            Cause::Association,
        ),
        ("/golden/point/0", json!("0"), Cause::Intersection),
        ("/golden/point/1", json!("2"), Cause::Intersection),
        ("/target/kind", json!("relative"), Cause::Target),
        ("/target/bits", json!(4097), Cause::TargetBits),
        (
            "/expected",
            json!({"refusal":"outside-domain"}),
            Cause::Expected,
        ),
    ] {
        let mut value = single();
        *value["entries"][0]
            .pointer_mut(path)
            .expect("mutation path") = replacement;
        rejects(&value, cause);
    }
    let mut value = single();
    value["entries"][0]["width_goal"]["bits"] = json!(200);
    value["routes"][0]["precision_bits"] = json!(200);
    rejects(&value, Cause::Width);
}

/// Judgment spellings, including escaped keys, are bounded before allocating JSON.
/// # Panics
/// Panics only if a new mathematical position bypasses the preallocation ceiling.
/// ´claim:docgen:constant-golden-preflight´
/// ´test:unit:constant-golden-preflight´
#[test]
fn constant_golden_preflight() {
    let mut value = single();
    value["entries"][0]["judgment"]["interval"][0] = json!("7".repeat(500_000));
    let bytes = serde_json::to_vec(&value).expect("encoding");
    let mut audit = crate::golden::json::Audit::default();
    assert!(matches!(
        crate::golden::json::parse(&bytes, Limits::default(), &mut audit),
        Err(Error::Invalid {
            cause: Cause::Literal,
            ..
        })
    ));
    assert_eq!(audit.tree_decodes, 0);
    let escaped = br#"{"entries":[{"judg\u006dent":{"inter\u0076al":["\u0031\u0032","3"]}}]}"#;
    let mut audit = crate::golden::json::Audit::default();
    assert!(matches!(
        crate::golden::json::parse(
            escaped,
            Limits {
                literal_bytes: 1,
                ..Limits::default()
            },
            &mut audit
        ),
        Err(Error::Invalid {
            cause: Cause::Literal,
            ..
        })
    ));
    assert_eq!(audit.tree_decodes, 0);
}

/// Individually valid rows sharing an original id cannot change its retained binding.
/// # Panics
/// Panics only if a changed binding admits or its isolated positive row refuses.
/// ´claim:docgen:constant-golden-shared-identity´
/// ´test:unit:constant-golden-shared-identity´
#[test]
fn constant_golden_shared_identity() {
    admits(&constants());
    for change in ["context", "judgment", "evidence", "golden"] {
        let mut value = geometry(160, [[0, 1], [0, 2], [0, 1]], 160);
        let first = value["entries"][0].clone();
        let row = &mut value["entries"][0];
        match change {
            "context" => {
                row["context"]["id"] = json!("other-context");
                row["judgment"]["context"] = json!("other-context");
                row["evidence"][0]["context"] = json!("other-context");
            }
            "judgment" => row["judgment"]["scope_hex"] = json!("00"),
            "evidence" => row["evidence"][0]["routes"][1]["interval"][1] = json!("1"),
            _ => {
                let other = geometry(160, [[1, 2], [1, 3], [1, 2]], 160);
                *row = other["entries"][0].clone();
            }
        }
        row["id"] = json!("other-row");
        admits(&value);
        value["entries"]
            .as_array_mut()
            .expect("entries")
            .insert(0, first);
        rejects(&value, Cause::Association);
    }
}
