# SPDX-License-Identifier: AGPL-3.0-only
"""Reproduce only the directed route for the twice-log-two golden values.

Use mpmath 1.3.0 revision b5c04506ef0cd4a1f1213f8389ee21c9c3551582.
No CAS interval, intersection, helper or export implementation is consulted.
--emit writes the separately evaluated route; otherwise check its retained rows.
"""
import json
import sys
from fractions import Fraction as F
from pathlib import Path
import mpmath
from mpmath import iv

assert mpmath.__version__ == "1.3.0"


def rational(t):
    """Decode a directed dyadic endpoint using integers only."""
    sign, mantissa, exponent, _ = t
    return (-1 if sign else 1) * F(mantissa) * F(2) ** exponent


def evaluate(digits):
    """The factor and argument are exact singleton intervals."""
    iv.dps = digits
    two = iv.mpf(2)
    assert tuple(map(rational, two._mpi_)) == (F(2), F(2))
    return tuple(map(rational, (two * iv.ln(two))._mpi_))


coarse = evaluate(80)
fine = evaluate(150)
assert coarse[0] <= fine[0] <= fine[1] <= coarse[1]
assert fine[1] - fine[0] <= F(1, 2**120)
result = [str(x) for x in fine]
if sys.argv[1:] == ["--emit"]:
    print(json.dumps({"interval": result, "precision_decimal_digits": 150}))
else:
    assert not sys.argv[1:]
    payload = json.loads(Path("data/golden/exact-constant.json").read_text())
    for row in payload["entries"]:
        assert row["expression"]["syntax"] == "Mul(2, Log(2))"
        assert row["evidence"][0]["routes"][1]["interval"] == result
    print(json.dumps({"directed_rows": len(payload["entries"]), "decimal_digits": 150}))
