use std::collections::HashMap;
use std::fs;

// Set of 95 valid characters for string generation
const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!@#$%^&*()-_+=[]{}\\|;:'\",.<>?/ ";

/// Simple 32-bit polynomial rolling hash (base 31, seed 0)
/// Equivalent to: h = ((h * 31) + byte) with wrap-around, truncated to 32 bits
fn hash(s: &[u8]) -> u32 {
    let mut h = 0u64;
    for &b in s {
        h = (h << 5).wrapping_sub(h).wrapping_add(b as u64);
    }
    h as u32
}

fn main() -> std::io::Result<()> {
    // Birthday paradox attack: collision expected after 65k attempts
    let mut seen = HashMap::new();
    let mut x = 0u64;

    loop {
        x += 1;
        // Generates 8-byte string using Linear Congruential Generator pseudo-random
        // Extracts different bytes from x * constant using bit shifting
        // Transforms x into 8 pseudo-random indices to select characters from CHARS.
        let s = (0..8).fold([0u8; 8], |mut s, i| {
            let idx = (x.wrapping_mul(0x517cc1b727220a95) >> (i * 8)) as usize % CHARS.len();
            s[i] = CHARS[idx];
            s
        });

        let h = hash(&s);

        // Detect collision: different strings with same hash
        if let Some(prev) = seen.insert(h, s) {
            if prev != s {
                let a = String::from_utf8_lossy(&s);
                let b = String::from_utf8_lossy(&prev);
                return fs::write("solutions/exercise04.txt", format!("{a},{b}\n"));
            }
        }
    }
}
