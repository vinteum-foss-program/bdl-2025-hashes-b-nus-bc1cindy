fn main() {
    let a = "aaaabbbb";
    let b = "bbbbaaaa";
    let h = |s: &str| {
        s.chars() // iter chars
            .enumerate()
            .fold(0u32, |h, (i, c)| h ^ (c as u32) << i % 4 * 8) // i = index, c = char
    };
    assert_eq!(h(a), h(b));
}