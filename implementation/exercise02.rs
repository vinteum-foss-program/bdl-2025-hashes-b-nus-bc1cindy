use std::fs;

fn main() -> std::io::Result<()> {
    // XOR32 hash: bytes XORed into 32-bit digest with cyclic 4-byte positioning
    let h = |s: &str| {
        s.bytes()
            .enumerate()
            .fold(0u32, |h, (i, b)| h ^ (b as u32) << i % 4 * 8)
    };
    let target_hash = h("bitcoin0");

    // Test 15 swap combinations
    // Mask bits indicate which pairs to swap
    let solution = (1..16)
        .filter_map(|m| {
            let mut b = "bitcoin0".bytes().collect::<Vec<_>>();
            (0..4).for_each(|i| {
                if m & 1 << i != 0 {
                    // Swap if bit i is set (= 1)
                    b.swap(i, i + 4)
                }
            });
            String::from_utf8(b).ok()
        })
        .find(|s| h(s) == target_hash)
        .expect("No collision found");

    fs::write("solutions/exercise02.txt", solution)?;
    Ok(())
}
