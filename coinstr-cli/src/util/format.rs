// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn format_number() {
        assert_eq!(number(100), "100".to_string());
        assert_eq!(number(1000), "1 000".to_string());
        assert_eq!(number(10000), "10 000".to_string());
        assert_eq!(number(100000), "100 000".to_string());
        assert_eq!(number(1000000), "1 000 000".to_string());
        assert_eq!(number(1000000000), "1 000 000 000".to_string());
    }
}
