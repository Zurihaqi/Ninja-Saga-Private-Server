//! Legacy client crypto; sessions remain server-generated and ownership is checked in persistence.
use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use base64::{Engine, engine::general_purpose::STANDARD};
use md5::Md5;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub fn decrypt_password(encrypted: &str, key: &str, iv: &str) -> Option<String> {
    let mut aes_key = [0; 16];
    let bytes = key.as_bytes();
    let n = bytes.len().min(16);
    aes_key[..n].copy_from_slice(&bytes[..n]);
    // PHP pads the decimal IV text, then OpenSSL consumes its first 16 bytes.
    let mut padded = iv.as_bytes().to_vec();
    let pad = 16 - padded.len() % 16;
    padded.extend(std::iter::repeat_n(pad as u8, pad));
    let data = STANDARD.decode(encrypted.trim()).ok()?;
    let decoded = cbc::Decryptor::<aes::Aes128>::new_from_slices(&aes_key, &padded[..16])
        .ok()?
        .decrypt_padded_vec_mut::<Pkcs7>(&data)
        .ok()?;
    String::from_utf8(decoded).ok()
}

pub fn random_hex() -> Result<String, getrandom::Error> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub fn mission_token() -> Result<String, getrandom::Error> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut result = String::with_capacity(10);
    while result.len() < 10 {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes)?;
        for byte in bytes {
            if byte < 248 {
                result.push(ALPHABET[(byte % 62) as usize] as char);
                if result.len() == 10 {
                    break;
                }
            }
        }
    }
    Ok(result)
}

pub fn hashes(owner: u64, raw: &str) -> (String, String) {
    let input = format!("{owner}{raw}");
    (
        format!("{:x}", Sha256::digest(input.as_bytes())),
        format!("{:x}", Md5::digest(input.as_bytes())),
    )
}

pub fn matches_session(owner: u64, stored: &str, supplied: &str) -> bool {
    let stored = stored.trim();
    let supplied = supplied.trim();
    if stored.is_empty() || supplied.is_empty() {
        return false;
    }
    let (sha, md5) = hashes(owner, stored);
    let decoded = STANDARD.decode(supplied).ok();
    [stored.as_bytes(), sha.as_bytes(), md5.as_bytes()]
        .into_iter()
        .any(|candidate| bool::from(candidate.ct_eq(supplied.as_bytes())))
        || decoded
            .as_ref()
            .is_some_and(|value| bool::from(stored.as_bytes().ct_eq(value)))
}

pub fn registration_hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

pub fn matches_registration_session(owner: u64, stored: &str, supplied: &str) -> bool {
    matches_session(owner, stored, supplied)
        || (!stored.trim().is_empty()
            && bool::from(
                registration_hash(stored.trim())
                    .as_bytes()
                    .ct_eq(supplied.trim().as_bytes()),
            ))
}

pub fn valid_legacy_hash(hash: &str, payload: &str) -> bool {
    let hash = hash.trim();
    match hash.len() {
        32 => format!("{:x}", Md5::digest(payload.as_bytes())) == hash,
        40 => format!("{:x}", sha1::Sha1::digest(payload.as_bytes())) == hash,
        64 => format!("{:x}", Sha256::digest(payload.as_bytes())) == hash,
        _ => true,
    }
}

pub fn base64(value: impl AsRef<[u8]>) -> String {
    STANDARD.encode(value)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, 12)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn php_aes_fixture_and_invalid_ciphertext() {
        // openssl_encrypt PHP 8.5.11: fixed public test password/key and decimal IV.
        let wire = "jo+n7pAgPZwhDMCN9reD8w==";
        assert_eq!(
            decrypt_password(wire, "0123456789abcdef", "123456").as_deref(),
            Some("NinjaTest123!")
        );
        assert!(decrypt_password("invalid", "0123456789abcdef", "123456").is_none());
        assert!(decrypt_password(wire, "wrong key", "123456").is_none());
    }
    #[test]
    fn sessions_accept_legacy_forms_but_require_the_owner() {
        let raw = "0123456789abcdef0123456789abcdef";
        let (sha, md5) = hashes(7, raw);
        for key in [raw.to_owned(), sha.clone(), md5, base64(raw)] {
            assert!(matches_session(7, raw, &key));
        }
        assert!(!matches_session(8, raw, &sha));
        assert!(!matches_session(7, "", &hashes(7, "").0));
        assert!(!matches_session(7, raw, "bad"));
        let registration = registration_hash(raw);
        assert!(matches_registration_session(7, raw, &registration));
        assert!(!matches_session(7, raw, &registration));
        assert!(!matches_registration_session(7, "", &registration_hash("")));
    }
}
