// SPDX-License-Identifier: AGPL-3.0-only
//! Independent unbounded integer fractions preserve signed and reciprocal pi oracle range.

use super::TestResult;
use crate::BinaryElementwise::{Product, Quotient};
use crate::loan::application::tests::binary::oracle::Rational;
use nrz_scalar::{Precision, Scalar};
use std::cmp::Ordering;

/// Literal decimal brackets are independent of production coefficient lowering.
fn pi() -> TestResult<[Rational; 2]> {
    Ok([
        Rational::fraction(3_141_592_653_589_793_238, 1_000_000_000_000_000_000)?,
        Rational::fraction(3_141_592_653_589_793_239, 1_000_000_000_000_000_000)?,
    ])
}

/// Native roundoff and coefficient width for these inputs below 25 fit these absolute ceilings.
fn slack<P: Scalar>() -> TestResult<Rational> {
    Rational::fraction(
        1,
        match P::PRECISION {
            Precision::F32 => 10_000,
            Precision::F64 => 100_000_000_000,
        },
    )
}

/// Check both original deterministic and nominal-error intervals, then bound both radii above.
pub(super) fn check<P: Scalar>(
    actual: [f64; 3],
    original: usize,
    component: usize,
    kind: u8,
) -> TestResult {
    image::<P>(actual, original, component, kind, 0)
}

/// A second exact unit-scale/unit-offset operation gives the original law pi*x+2.
pub(super) fn composed<P: Scalar>(
    actual: [f64; 3],
    original: usize,
    component: usize,
) -> TestResult {
    image::<P>(actual, original, component, 1, 1)
}

/// Independent signed and inverse factors retain the full decimal pi bracket.
fn factors(kind: u8) -> TestResult<([Rational; 2], Rational)> {
    let pi = pi()?;
    let one = Rational::fraction(1, 1)?;
    let minus = Rational::fraction(-1, 1)?;
    Ok(match kind {
        0 => ([one.clone(), one.clone()], one),
        1 => (pi.clone(), pi[1].clone()),
        2 => (
            [pi[1].image(&minus, Product)?, pi[0].image(&minus, Product)?],
            pi[1].clone(),
        ),
        3 => (
            [one.image(&pi[1], Quotient)?, one.image(&pi[0], Quotient)?],
            one.image(&pi[0], Quotient)?,
        ),
        _ => return Err("coefficient oracle".into()),
    })
}

fn image<P: Scalar>(
    actual: [f64; 3],
    original: usize,
    component: usize,
    kind: u8,
    shift: i64,
) -> TestResult {
    let one = Rational::fraction(1, 1)?;
    let (factors, magnitude) = factors(kind)?;
    let offsets = if kind < 2 { [one.clone(), one] } else { pi()? };
    let nominal = Rational::fraction(i64::try_from(10 + component * 10 + original)?, 1)?;
    let total = Rational::fraction(i64::try_from(3 + component)?, 1)?;
    let prior = Rational::fraction(i64::try_from(1 + component)?, 1)?;
    for factor in &factors {
        for offset in &offsets {
            let offset = offset.add(&Rational::fraction(shift, 1)?);
            for sign in [-1, 1] {
                let signed = Rational::fraction(sign, 1)?;
                let endpoint = nominal
                    .add(&total.image(&signed, Product)?)
                    .image(factor, Product)?
                    .add(&offset);
                covered(&endpoint, actual[0], actual[1])?;
                let nominal_endpoint = nominal
                    .add(&prior.image(&signed, Product)?)
                    .image(factor, Product)?
                    .add(&offset);
                covered(&nominal_endpoint, actual[0], actual[2])?;
            }
        }
    }
    at_most(
        actual[1],
        &total.image(&magnitude, Product)?.add(&slack::<P>()?),
    )?;
    at_most(
        actual[2],
        &prior.image(&magnitude, Product)?.add(&slack::<P>()?),
    )
}

/// Compare exact binary endpoints; the assertion never rounds centre plus or minus radius.
fn covered(value: &Rational, center: f64, radius: f64) -> TestResult {
    let center = Rational::binary(center)?;
    let radius = Rational::binary(radius)?;
    assert_ne!(center.sub(&radius).compare(value), Ordering::Greater);
    assert_ne!(value.compare(&center.add(&radius)), Ordering::Greater);
    Ok(())
}

fn at_most(value: f64, ceiling: &Rational) -> TestResult {
    assert_ne!(Rational::binary(value)?.compare(ceiling), Ordering::Greater);
    Ok(())
}

/// Exact exponent rescaling reduces tiny and large native results to pi*[3/4,5/4].
pub(super) fn normalized<P: Scalar>(actual: [f64; 3]) -> TestResult {
    let pi = pi()?;
    for factor in &pi {
        for numerator in [3, 5] {
            covered(
                &factor.image(&Rational::fraction(numerator, 4)?, Product)?,
                actual[0],
                actual[1],
            )?;
        }
        covered(factor, actual[0], actual[2])?;
    }
    at_most(
        actual[1],
        &pi[1]
            .image(&Rational::fraction(1, 4)?, Product)?
            .add(&slack::<P>()?),
    )?;
    at_most(actual[2], &slack::<P>()?)
}
