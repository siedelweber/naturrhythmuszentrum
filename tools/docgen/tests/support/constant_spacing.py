# SPDX-License-Identifier: AGPL-3.0-only
"""Independent integer/rational neighbour decoding for the constant golden values.

Reads the golden payload, not CAS or mpmath helpers and not a native export.
No float conversion or transcendental evaluation participates in these checks.
"""
import json
from fractions import Fraction as F
from pathlib import Path


def decode(bits):
    """IEEE binary64 normal significand and exponent decoded as integers."""
    sign = -1 if bits >> 63 else 1
    exponent = (bits >> 52) & 2047
    assert 0 < exponent < 2047
    return sign * F(2**52 + (bits & (2**52 - 1))) * F(2) ** (exponent - 1075)


below = 0x3ff62e42fefa39ef
down, up = decode(below), decode(below + 1)
assert up - down == F(1, 2**52)
payload = json.loads(Path('data/golden/exact-constant.json').read_text())
rows = payload['entries']
assert [r['target'] for r in rows] == [{'kind': 'absolute', 'bits': 40}, {'kind': 'absolute', 'bits': 60}]
assert [r['expected'] for r in rows] == ['enclosure', {'refusal': 'target-not-certifiable'}]
for row in rows:
    a, b = [tuple(map(F, r['interval'])) for r in row['evidence'][0]['routes']]
    lo, hi = map(F, row['golden']['point'])
    assert (lo, hi) == (max(a[0], b[0]), min(a[1], b[1]))
    assert lo <= hi and hi - lo <= F(1, 2**120)
    assert 1 < down < lo <= hi < up < 2
    assert lo - down > F(1, 2**128) and up - hi > F(1, 2**128)
    assert hi - down <= F(1, 2**40) and up - lo <= F(1, 2**40)
    # One endpoint already exceeds the 53-bit allowance, hence every tighter one.
    assert lo - down > F(1, 2**53) or up - hi > F(1, 2**53)
    assert up - down > F(2, 2**60)
print(json.dumps({'rows': len(rows), 'lower_bits': hex(below), 'upper_bits': hex(below + 1),
                  'lower': str(down), 'upper': str(up), 'refuses_from_bits': 53}))
