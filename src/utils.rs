pub fn u64_to_digit(number: u64, digit_count: u32) -> Vec<u32> {
    let mut number = number.to_string();
    let number_digits = number.len() as u32;
    let extra_digits = digit_count.saturating_sub(number_digits);

    for _ in 0..extra_digits {
        number.insert(0, '0');
    }

    number
        .chars()
        .map(|c| c.to_digit(10).unwrap() as u32)
        .collect()
}
