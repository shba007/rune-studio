use crate::settings::KEYRING_SERVICE;

const KEYRING_USER: &str = "agent-bridge-token";

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|err| err.to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn get_or_create() -> Result<String, String> {
    let entry = entry()?;
    if let Ok(token) = entry.get_password()
        && !token.is_empty()
    {
        return Ok(token);
    }
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|err| err.to_string())?;
    let token = hex(&bytes);
    entry.set_password(&token).map_err(|err| err.to_string())?;
    Ok(token)
}

pub fn matches(candidate: &str) -> bool {
    let Ok(expected) = get_or_create() else {
        return false;
    };
    constant_time_eq(candidate.as_bytes(), expected.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b)
        .fold(0_u8, |difference, (x, y)| difference | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_accepts_only_identical_tokens() {
        assert!(constant_time_eq(b"abc123", b"abc123"));
        assert!(!constant_time_eq(b"abc123", b"abc124"));
        assert!(!constant_time_eq(b"abc", b"abc123"));
        assert!(!constant_time_eq(b"", b"x"));
    }

    #[test]
    fn hex_is_two_digits_per_byte() {
        assert_eq!(hex(&[0, 15, 255]), "000fff");
    }
}
