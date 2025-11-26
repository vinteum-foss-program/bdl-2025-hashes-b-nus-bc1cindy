use sha2::{Digest, Sha256};

/// Finds a string that produces a SHA-256 hash starting with the target prefix.
/// Iterates through numbers appended to the prefix until a match is found.
fn find_collision(prefix: &str, target: &str) -> String {
    (0..)
        .map(|n| format!("{}{}", prefix, n))
        // Test each candidate until hash matches target prefix
        .find(|s| {
            // Compute SHA-256 hash as hex string
            let hash = format!("{:x}", Sha256::digest(s.as_bytes()));
            hash.starts_with(target)
        })
        .unwrap()
}

fn main() {
    // Define target prefixes and their bit-length difficulty
    let targets = [("cafe", 16), ("faded", 20), ("decade", 24)];

    // Find collision for each target
    let solutions: Vec<_> = targets
        .iter()
        .map(|(target, bits)| {
            // Verify and display the solution
            println!("Searching {}-bit collision ({}...)...", bits, target);
            let sol = find_collision("bitcoin", target);
            println!("Found: {} -> {:x}", sol, Sha256::digest(sol.as_bytes()));
            sol
        })
        .collect();

    // Save results as comma-separated values
    let output = solutions.join(",");
    std::fs::write("solutions/exercise06.txt", &output).unwrap();
    println!("\n✅ Saved in solutions/exercise06.txt:\n{}", output);
}
