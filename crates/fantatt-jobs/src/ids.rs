//! Ledger and job identifiers: canonical ULIDs from the OS random source.
//! why: the schema asserts the ULID shape on every ingest id column and the
//! workspace carries no ULID crate, so the Crockford encoding lives here.

use rand::RngCore;

/// Crockford base32 without the ambiguous I, L, O, and U letters.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Length of a canonical ULID body.
const LENGTH: usize = 26;

/// Bits one Crockford character carries.
const SYMBOL_BITS: u32 = 5;

/// A fresh canonical ULID: 128 random bits in 26 Crockford characters.
pub fn ulid() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    let value = u128::from_be_bytes(bytes);
    let mut body = String::with_capacity(LENGTH);
    for index in 0..LENGTH {
        let shift = (LENGTH as u32 - 1 - index as u32) * SYMBOL_BITS;
        let symbol = ((value >> shift) as usize) & 0x1F;
        body.push(ALPHABET[symbol] as char);
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ulid_is_26_crockford_characters() {
        let id = ulid();
        assert_eq!(id.len(), LENGTH);
        assert!(
            id.chars().all(|c| ALPHABET.contains(&(c as u8))),
            "{id} carries a non-Crockford character"
        );
    }

    #[test]
    fn two_ulids_do_not_collide() {
        assert_ne!(ulid(), ulid());
    }
}
