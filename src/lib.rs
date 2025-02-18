// rust-shamir implements Shamir's secret sharing for arbitrarily sized secrets.
// This is accomplished by using a new polynomial per byte, over the Galois
// field GF(2^8). (t,n) are configurable; t is the minimum threshold required to
// rebuild the secret and n is the number of shares to distribute.

mod gf;

extern crate rand;
extern crate zeroize;

use gf::GfOps;
use rand::Rng;
use std::cmp;
use zeroize::Zeroize;

// SharePoint defines a share for a particular byte. It is a point (x, y) on the
// sharing polynomial.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SharePoint {
    x: gf::GF256e,
    y: gf::GF256e,
}

#[derive(Debug, PartialEq)]
pub enum SecretSharingError {
    TorNisZero,
    MissingShareForByte,
}

pub type Shares = Vec<SharePoint>;

// share_value shares a single `secret_byte` with Shamir's using parameters
// (t,n). An entirely random polynomial is created with degree t-1 such that `t`
// shares are required to reconstruct the secret.
fn share_value(t: u8, n: u8, secret_byte: &u8) -> Shares {
    let mut rng = rand::thread_rng();

    // pull random coefficients for the polynomial.
    // since we're operating in GF(2^8), the coefficients are conveniently byte-aligned.
    let coeff: Vec<(gf::GF256e, gf::GF256e)> = vec![0; n as usize]
        .iter()
        .enumerate()
        .map(|(i, _)| (rng.gen(), cmp::max((t - 1).saturating_sub(i as u8), 1)))
        .collect();

    // construct the polynomial
    // f(x) = mx^t-1 + m2x^t-2 ... + b
    let p = |x: gf::GF256e| {
        coeff
            .iter()
            .fold(0, |y, m| y.add(m.0.mul(x.exp(m.1))))
            .add(*secret_byte)
    };

    // split the secret for x = 1..n
    vec![0; n as usize]
        .iter()
        .enumerate()
        .map(|(i, _)| SharePoint {
            x: i as gf::GF256e + 1,
            y: p(i as gf::GF256e + 1),
        })
        .collect()
}

// construct_shares creates a new Share of the supplied `secret`. It returns a
// Vec<Share>, where each vec of shares belings to participant 1 -> n. t shares
// are required to reconstruct the secret. `secret` is an arbitrary size byte
// slice.
pub fn construct_shares(t: u8, n: u8, secret: &[u8]) -> Result<Vec<Shares>, SecretSharingError> {
    if t == 0 || n == 0 {
        return Err(SecretSharingError::TorNisZero);
    }

    let mut shares: Vec<Shares> = Vec::new();
    for _ in 0..n {
        shares.push(Vec::new());
    }

    let shares =
        secret
            .iter()
            .map(|b| share_value(t, n, b))
            .fold(shares, |mut s, share_bytes| {
                for (i, b) in share_bytes.iter().enumerate() {
                    s[i].push(SharePoint { x: b.x, y: b.y });
                }
                s
            });

    Ok(shares)
}

// lagrange_interpolate computes the lagrange polynomial from the supplied
// shares, then returns the value of the interpolated polynomial at `x`.
fn lagrange_interpolate(shares: Vec<&SharePoint>, x: gf::GF256e) -> gf::GF256e {
    shares.iter().fold(0 as gf::GF256e, |y, j| {
        let phi = shares
            .iter()
            .filter(|m| m.x != j.x)
            .fold(1 as gf::GF256e, |phi, m| {
                phi.mul(x.sub(m.x).div(j.x.sub(m.x)))
            });

        y.add(j.y.mul(phi))
    })
}

fn reconstruct_value(shares: Vec<&SharePoint>) -> gf::GF256e {
    lagrange_interpolate(shares, 0)
}

// reconstruct takes a slice of shares and attempts to reconstruct the shared
// secret. The reconstruction is not verifiable; reconstructing invalid shares
// will return an invalid secret, not an error.
pub fn reconstruct(shares: Vec<Shares>) -> Result<Vec<u8>, SecretSharingError> {
    // ensure the blobs are the same length
    let sz = shares[0].len();
    let all_same_len = shares.iter().all(|share| share.len() == sz);
    if !all_same_len {
        return Err(SecretSharingError::MissingShareForByte);
    }

    let result = vec![0; sz as usize]
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let byte_shares = shares.iter().map(|share| &share[i]);
            reconstruct_value(byte_shares.collect())
        })
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vec_eq<T: PartialEq>(a: &Vec<T>, b: &Vec<T>) -> bool {
        let matchcount = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
        matchcount == a.len() && matchcount == b.len()
    }

    #[test]
    fn test_share_construct() {
        let secret = vec![0xfe, 0xff, 0xaf, 0xbe];
        let shares = construct_shares(3, 5, &secret).unwrap();
        assert_eq!(shares.len(), 5);
    }
    #[test]
    fn test_share_construct_reconstruct() {
        let secret = vec![
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xf, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
        ];
        let shares = construct_shares(3, 5, &secret).unwrap();
        assert_eq!(shares.len(), 5);

        let reconstructed = reconstruct(shares).unwrap();
        assert!(vec_eq(&reconstructed, &secret));
    }
    #[test]
    fn test_share_construct_reconstruct_shares_omitting() {
        let secret = vec![
            0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
            0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xfe, 0xed,
        ];
        let t = 3;
        let n = 5;
        let mut todelete = 2;
        let mut shares = construct_shares(t, n, &secret).unwrap();
        for _ in 0..todelete {
            shares.pop();
        }
        let reconstructed = reconstruct(shares);
        assert!(vec_eq(&reconstructed.unwrap(), &secret));

        todelete = 3;
        shares = construct_shares(t, n, &secret).unwrap();
        for _ in 0..todelete {
            shares.pop();
        }
        let reconstructed_bad = reconstruct(shares);
        assert!(!vec_eq(&reconstructed_bad.unwrap(), &secret));
    }
}
