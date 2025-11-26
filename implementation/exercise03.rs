use std::fs;

fn main() -> std::io::Result<()> {
    // hash function: XOR bytes with position-based bit shifts
    let h = |s: &[u8]| {
        s.iter()
            .enumerate()
            .fold(0u32, |h, (i, &b)| h ^ (b as u32) << (i % 4 * 8))
    };

    // Target hash value as little-endian bytes
    let t = 0x1b575451u32.to_le_bytes();

    // Brute-force search through printable ASCII combinations
    for b0 in 33..127 {
        // First byte: no leading space
        for b1 in 32..127 {
            // Remaining bytes: allow space
            for b2 in 32..127 {
                for b3 in 32..127 {
                    for b4 in 32..127 {
                        for b5 in 32..127 {
                            // Construct first 8 bytes: 6 variable + 2 spaces
                            let base = [b0, b1, b2, b3, b4, b5, b' ', b' '];

                            // Calculate last 4 bytes using XOR to match target hash
                            let end = [
                                base[0] ^ base[4] ^ t[0],
                                base[1] ^ base[5] ^ t[1],
                                base[2] ^ base[6] ^ t[2],
                                base[3] ^ base[7] ^ t[3],
                            ];

                            // Only proceed if calculated bytes are printable (no space/control chars)
                            if end.iter().all(|&b| (33..127).contains(&b)) {
                                // Concatenate to form 12-byte candidate
                                let bytes = [&base[..], &end[..]].concat();

                                // Verify hash matches target
                                if h(&bytes) == 0x1b575451 {
                                    return fs::write("solutions/exercise03.txt", bytes);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
