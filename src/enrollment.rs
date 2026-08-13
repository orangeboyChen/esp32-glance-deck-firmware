use hex::ToHex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnrollmentSession {
    pub pairing_code: String,
    pub claim_secret: String,
}

impl EnrollmentSession {
    pub fn from_random(random: [u8; 32]) -> Self {
        let code_source = u32::from_le_bytes(random[0..4].try_into().expect("four bytes"));
        Self {
            pairing_code: format!("{:06}", 100_000 + code_source % 900_000),
            claim_secret: random.encode_hex::<String>(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.pairing_code.len() != 6
            || !self.pairing_code.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("pairing_code_invalid");
        }
        if self.claim_secret.len() != 64
            || !self
                .claim_secret
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err("claim_secret_invalid");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_a_six_digit_code_and_256_bit_secret() {
        let session = EnrollmentSession::from_random([0xff; 32]);
        assert_eq!(session.pairing_code.len(), 6);
        assert_eq!(session.claim_secret.len(), 64);
        assert_eq!(session.validate(), Ok(()));
    }

    #[test]
    fn rejects_noncanonical_secret() {
        let session = EnrollmentSession {
            pairing_code: "123456".to_owned(),
            claim_secret: "A".repeat(64),
        };
        assert_eq!(session.validate(), Err("claim_secret_invalid"));
    }
}
