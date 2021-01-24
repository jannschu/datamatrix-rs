"""
Implementation of some GF(256) arithmetic used in Data Matrix.
"""

# This is the polynomical used by Data Matrix do define multiplication.
# QR codes use a different polynomial for example.
IRREDUCIBLE_P = 0b1_0010_1101

class GF:
    """Representation of an element from GF(256)."""

    def __init__(self, v):
        if isinstance(v, GF):
            self.v = v.v
        else:
            self.v = int(v)
        assert 0 <= int(self.v) < 256

    def __add__(self, o):
        o = GF(o).v
        return GF(self.v ^ o)

    def __sub__(self, o):
        return self + o

    def __eq__(self, o):
        return self.v == GF(o).v

    def __pow__(self, p):
        if self == 0:
            return GF(0)
        if int(p) == 1:
            return GF(1)
        i = LOG[self.v]
        i = (i * int(p)) % 255
        return GF(ANTI_LOG[i])

    def mul_slow(self, b):
        a = self.v
        b = GF(b).v
        p = 0
        for i in range(8):
            if b & 1:
                p ^= a
            b >>= 1
            carry = bool(a & 0b1000_0000)
            a = (a << 1) & 0xFF
            if carry:
                a ^= (IRREDUCIBLE_P & 0xFF)
        return GF(p)

    def __mul__(self, b):
        if self == 0 or b == 0:
            return GF(0)
        ia = LOG[self.v]
        ib = LOG[GF(b).v]
        return GF(ANTI_LOG[(ia + ib) % 255])

    def __truediv__(self, b):
        assert b != 0
        if self == 0:
            return GF(0)
        ia = LOG[self.v]
        ib = LOG[b.v]
        i = ia - ib
        if i < 0:
            i += 255
        return GF(ANTI_LOG[i])

    def __repr__(self):
        return f"GF({self.v})"


class Poly:
    """Polynomial over GF(256)."""

    def __init__(self, coeffs):
        """Create polynomial from coefficients (order: low to high power)."""
        if isinstance(coeffs, Poly):
            self.coeffs = coeffs.coeffs[:]
        else:
            self.coeffs = list(coeffs)
        while self.coeffs and self.coeffs[-1] == 0:
            self.coeffs.pop()

    def __mul__(self, o):
        """Multiply two polynomicals"""
        other = Poly(o).coeffs
        highest_power = len(self.coeffs) - 1 + len(other) - 1
        res = [0 for _ in range(highest_power + 1)]
        for power_a, a in enumerate(self.coeffs):
            for power_b, b in enumerate(other):
                res[power_a + power_b] = a * b + res[power_a + power_b]
        return Poly(res)

    def __call__(self, x):
        s = GF(0)
        x_pow = GF(1)
        for cf in self.coeffs:
            s += cf * x_pow
            x_pow *= x
        return s

    def __add__(self, o):
        new_len = max(len(o.coeffs), len(self.coeffs))
        new = [GF(0)] * new_len
        for i in range(new_len):
            if i < len(o.coeffs):
                new[i] += o.coeffs[i]
            if i < len(self.coeffs):
                new[i] += self.coeffs[i]
        return Poly(new)

    def __sub__(self, o):
        return self + o

    @property
    def deg(self):
        return len(self.coeffs) - 1
    
    def euclid_div(self, b):
        q = Poly([GF(0)])
        r = Poly(self.coeffs)
        c = b.coeffs[-1]
        while r.deg >= b.deg:
            s_coeff = [GF(0)] * (r.deg - b.deg + 1)
            s_coeff[-1] = r.coeffs[-1] / c
            s = Poly(s_coeff)
            q += s
            r -= s * b
        return q, r

    def __eq__(self, o):
        return self.coeffs == Poly(o).coeffs

    def __repr__(self):
        p = []
        for i in range(len(self.coeffs), 0, -1):
            c = self.coeffs[i - 1] 
            if c != 0:
                if i == 1:
                    p.append(f"{c.v}")
                else:
                    if c == 1:
                        p.append(f"x^{i - 1}")
                    else:
                        p.append(f"{c.v}x^{i - 1}")
        return " + ".join(p)


assert GF(0x53) + GF(0xCA) == GF(0x99)

ANTI_LOG = [None] * 256
LOG = [None] * 256

# Populate the tables
p = GF(1)
for i in range(0, 256):
    ANTI_LOG[i] = p
    LOG[p.v] = i
    p = p.mul_slow(2)

# GF(2), GF(1),
# GF(5), GF(2)
# rhs: GF(56), GF(23)
x = [GF(183), GF(246)]
print("row1", GF(2) * x[0] + GF(1) * x[1])
print("row2", GF(5) * x[0] + GF(2) * x[1])

print("tmp", GF(2) * GF(2))

# print(ANTI_LOG)

def gen(n):
    """
    Compute the n-th generator polynomial.

    That is, compute (x + 2 ** 1) * (x + 2 ** 2) * ... * (x + 2 ** n).
    """
    p = Poly([GF(1)])
    two = GF(1)
    for i in range(1, n + 1):
        two *= 2
        p *= Poly([two, GF(1)])
    return p

data = Poly([GF(23), GF(40), GF(11)][::-1])
x_k = Poly([GF(i == 5) for i in range(6)])
p = data * x_k
g = gen(5)
q, r = p.euclid_div(g)
assert p == q * g + r
print("data:", data)
print("error_code:", r)


p = Poly([GF(90), GF(0), GF(23), GF(0), GF(1)][::-1])
print(p(2))
print(p(GF(2) ** 2))
print(p(GF(2) ** 3))

assert GF(2) ** 3 == GF(2) * GF(2) * GF(2)


# This will print the generating polynomials
# gens = [5, 7, 10, 11, 12, 14, 18, 20, 24, 28, 36, 42, 48, 56, 62, 68]
# print("Generating polynomials:")
# for g in gens:
#     cs = reversed(gen(g).coeffs)
#     print("[" + ", ".join(str(c.v) for c in cs) + "],")