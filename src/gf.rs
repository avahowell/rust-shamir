pub trait GfOps<T> {
    fn add(self, x: T) -> T;
    fn sub(self, x: T) -> T;
    fn mul(self, x: T) -> T;
    fn inv(self) -> T;
    fn div(self, x: T) -> T;
    fn exp(self, x: T) -> T;
}

pub type GF256e = u8;

// fully constant-time mplementation of GfOps for GF(2^8) with reduction
// polynomial 0x11b.
impl GfOps<GF256e> for GF256e {
    fn add(self, x: GF256e) -> GF256e {
        return self ^ x;
    }
    fn sub(self, x: GF256e) -> GF256e {
        return self ^ x;
    }
    fn mul(self, x: GF256e) -> GF256e {
        let mut yj: u16 = self as u16;
        let mut xj: u16 = x as u16;
        let mut z: u16 = 0;

        for _ in 0..8 {
            z ^= ((0 as u16).wrapping_sub(xj & 1)) & yj;
            xj >>= 1;
            yj <<= 1;
            yj ^= (0 as u16).wrapping_sub(yj >> 8) & 0x11b;
        }

        return z as GF256e;
    }
    fn div(self, x: GF256e) -> GF256e {
        return self.mul(x.inv());
    }
    fn exp(self, x: GF256e) -> GF256e {
        let mut r = 1;
        let mut q: GF256e = 0;
        for i in 0..255 {
            let mut mask = i ^ x;
            mask |=
                mask << 1 | mask << 2 | mask << 3 | mask << 4 | mask << 5 | mask << 6 | mask << 7;
            mask |=
                mask >> 1 | mask >> 2 | mask >> 3 | mask >> 4 | mask >> 5 | mask >> 6 | mask >> 7;
            q |= r & !mask;
            r = r.mul(self);
        }
        return q;
    }
    fn inv(self) -> GF256e {
        let mut j = self.mul(self);
        for _ in 0..6 {
            j = j.mul(self);
            j = j.mul(j);
        }
        return j;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let a: GF256e = 0xbe;
        let b: GF256e = 0x6c;
        assert_eq!(a ^ b, a.add(b));
    }
    #[test]
    fn test_sub() {
        let a: GF256e = 0xbe;
        let b: GF256e = 0x6c;
        assert_eq!(a ^ b, a.sub(b));
    }
    // TODO: embed correct table values
    #[test]
    fn test_mul() {
        let a: GF256e = 0xb6;
        let b: GF256e = 0x53;
        assert_eq!(a.mul(b), 0x36);
    }
    #[test]
    fn test_inv() {
        let a: GF256e = 0xcc;
        assert_eq!(a.mul(a.inv()), 0x1);
    }
    // TODO: embed correct table values
    #[test]
    fn test_exp() {
        assert_eq!((0x02 as GF256e).exp(0x04), 1 << 4);

        assert_eq!((0x12 as GF256e).exp(0), 1);
        assert_eq!((0x12 as GF256e).exp(1), 0x12);
    }
}
