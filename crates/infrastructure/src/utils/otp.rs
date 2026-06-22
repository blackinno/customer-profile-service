use rand::Rng;

/// Generate a zero-padded 6-digit OTP string.
pub fn generate_otp() -> String {
    format!("{:06}", rand::thread_rng().gen_range(0..=999999u32))
}

/// Generate a 6-character uppercase alphabetic reference code.
pub fn generate_ref_code() -> String {
    (0..6)
        .map(|_| char::from(rand::thread_rng().gen_range(b'A'..=b'Z')))
        .collect()
}
