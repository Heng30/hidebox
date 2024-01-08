use rand::Rng;

#[allow(dead_code)]
pub fn split_string_to_fixed_length_parts(input: &str, length: usize) -> Vec<String> {
    input
        .chars()
        .collect::<Vec<_>>()
        .chunks(length)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

#[allow(dead_code)]
pub fn random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = ('a'..='z').collect();
    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_string() {
        for i in 1..100 {
            assert_eq!(random_string(i).len(), i);
        }
    }
}

