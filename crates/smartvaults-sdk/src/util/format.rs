// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

const SCALES: [(u8, &str); 4] = [(1, "K"), (2, "M"), (3, "Bn"), (4, "T")];

pub fn number(num: u64) -> String {
    let mut number: String = num.to_string();

    if number.len() > 3 {
        let reversed: String = number.chars().rev().collect();
        number.clear();
        for (index, char) in reversed.chars().enumerate() {
            if index != 0 && index % 3 == 0 {
                number.push(' ');
            }
            number.push(char);
        }
        number = number.chars().rev().collect();
    }

    number
}

pub fn big_number(num: u64) -> String {
    let mut number: String = num.to_string();

    if number.len() > 3 {
        let mut prevpow: u64 = 1000;
        let mut prevscale: &str = "K";

        for scale in SCALES.iter() {
            let pow: u64 = u64::pow(1000, scale.0 as u32);
            if (num / pow) < 1 {
                break;
            }
            prevpow = pow;
            prevscale = scale.1;
        }

        number = format!("{}{}", (num as f32 / prevpow as f32).round(), prevscale);
    }

    number
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn format_number() {
        assert_eq!(number(100), "100".to_string());
        assert_eq!(number(1_000), "1 000".to_string());
        assert_eq!(number(10_000), "10 000".to_string());
        assert_eq!(number(100_000), "100 000".to_string());
        assert_eq!(number(1_000_000), "1 000 000".to_string());
        assert_eq!(number(1_000_000_000), "1 000 000 000".to_string());
    }

    #[test]
    fn format_big_number() {
        assert_eq!(big_number(100), "100".to_string());
        assert_eq!(big_number(1_000), "1K".to_string());
        assert_eq!(big_number(10_000), "10K".to_string());
        assert_eq!(big_number(100_000), "100K".to_string());
        assert_eq!(big_number(1_000_000), "1M".to_string());
        assert_eq!(big_number(1_000_000_000), "1Bn".to_string());
    }
}
