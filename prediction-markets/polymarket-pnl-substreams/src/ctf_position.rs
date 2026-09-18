// Gnosis Conditional Tokens position-ID derivation, ported from Polymarket's
// own reference implementation (github.com/Polymarket/go-ctf-utils,
// utils/positionid.go), not from the on-chain Solidity (which additionally
// supports combining with a non-zero parentCollectionId via a BN254 ecAdd
// precompile call — unneeded here since Polymarket never nests positions,
// and confirmed correct against that repo's own test vectors below).
//
// Algorithm: collectionId = keccak256(conditionId || indexSet), then walked
// forward one at a time until x^3 + 3 is a quadratic residue mod P (curve
// point exists), with a final parity-correction bit matching a "sign" bit
// taken from the original hash. positionId = keccak256(collateralToken ||
// collectionId), as a uint256.
use substreams::scalar::BigInt;
use std::str::FromStr;
use tiny_keccak::{Hasher, Keccak};

fn p() -> BigInt {
    BigInt::from_str(
        "21888242871839275222246405745257275088696311157297823662689037894645226208583",
    )
    .unwrap()
}

fn legendre_exponent() -> BigInt {
    // (P - 1) / 2
    BigInt::from_str(
        "10944121435919637611123202872628637544348155578648911831344518947322613104291",
    )
    .unwrap()
}

fn two_pow_254() -> BigInt {
    BigInt::from_str(
        "28948022309329048855892746252171976963317496166410141009864396001978282409984",
    )
    .unwrap()
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(data);
    let mut out = [0u8; 32];
    hasher.finalize(&mut out);
    out
}

fn addmod(a: &BigInt, b: &BigInt, m: &BigInt) -> BigInt {
    (a.clone() + b.clone()) % m.clone()
}

fn mulmod(a: &BigInt, b: &BigInt, m: &BigInt) -> BigInt {
    (a.clone() * b.clone()) % m.clone()
}

fn pow_mod(base: &BigInt, exp: &BigInt, m: &BigInt) -> BigInt {
    let mut result = BigInt::one();
    let mut base = base.clone() % m.clone();
    let mut e = exp.clone();
    let zero = BigInt::zero();
    let two = BigInt::from(2u32);
    while e > zero {
        if e.clone() % two.clone() != zero {
            result = mulmod(&result, &base, m);
        }
        base = mulmod(&base, &base, m);
        e = e / two.clone();
    }
    result
}

fn is_quadratic_residue(a: &BigInt, m: &BigInt) -> bool {
    pow_mod(a, &legendre_exponent(), m) == BigInt::one()
}

/// `index_set` as a big-endian 32-byte value (raw indexSet, e.g. 1, 2, 3... —
/// not an outcome slot index).
fn compute_collection_id(condition_id: &[u8; 32], index_set: &BigInt) -> [u8; 32] {
    let p_val = p();
    let mut payload = Vec::with_capacity(64);
    payload.extend_from_slice(condition_id);
    let (_, index_bytes) = index_set.to_bytes_be();
    let mut padded = vec![0u8; 32];
    let start = 32usize.saturating_sub(index_bytes.len());
    padded[start..].copy_from_slice(&index_bytes[index_bytes.len().saturating_sub(32)..]);
    payload.extend_from_slice(&padded);

    let hash = keccak256(&payload);
    let hash_int = BigInt::from_unsigned_bytes_be(&hash);
    let odd = (hash_int.clone() >> 255u32) != BigInt::zero();

    let mut x1 = hash_int;
    loop {
        x1 = addmod(&x1, &BigInt::one(), &p_val);
        let x1_sq = mulmod(&x1, &x1, &p_val);
        let x1_cubed = mulmod(&x1_sq, &x1, &p_val);
        let yy = addmod(&x1_cubed, &BigInt::from(3u32), &p_val);
        if is_quadratic_residue(&yy, &p_val) {
            break;
        }
    }

    if odd {
        let bit254 = two_pow_254();
        if (x1.clone() & bit254.clone()) == BigInt::zero() {
            x1 = x1 + bit254;
        } else {
            x1 = x1 - bit254;
        }
    }

    let (_, x1_bytes) = x1.to_bytes_be();
    let mut out = [0u8; 32];
    let start = 32usize.saturating_sub(x1_bytes.len());
    out[start..].copy_from_slice(&x1_bytes[x1_bytes.len().saturating_sub(32)..]);
    out
}

/// `collateral` is the 20-byte token address; returns the position ID
/// (Polymarket's on-chain token_id) as a decimal string.
pub fn compute_position_id(collateral: &[u8; 20], condition_id: &[u8; 32], index_set: &BigInt) -> String {
    let collection_id = compute_collection_id(condition_id, index_set);
    let mut payload = Vec::with_capacity(52);
    payload.extend_from_slice(collateral);
    payload.extend_from_slice(&collection_id);
    let hash = keccak256(&payload);
    BigInt::from_unsigned_bytes_be(&hash).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real test vectors from Polymarket/go-ctf-utils, utils/positionid_test.go.
    #[test]
    fn matches_go_ctf_utils_reference_vectors() {
        let condition_id: [u8; 32] =
            hex_literal::hex!("41771a29f1fa3b5ac743ddcf224017f802bd69152c1a65230ec666abfc22b708");
        let collateral: [u8; 20] = hex_literal::hex!("2791bca1f2de4661ed88a30c99a7a9449aa84174");

        let token0 = compute_position_id(&collateral, &condition_id, &BigInt::from(1u32));
        let token1 = compute_position_id(&collateral, &condition_id, &BigInt::from(2u32));

        assert_eq!(
            token0,
            "87848146419241057657677458104196204655830537958664996373577118668208015365957"
        );
        assert_eq!(
            token1,
            "11831001752042525186810643219442170205387399550590230096735412415464402686073"
        );
    }
}
