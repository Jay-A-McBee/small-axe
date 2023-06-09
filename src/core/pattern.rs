use std::collections::HashSet;

#[derive(Debug, Eq, PartialEq)]
pub enum PatternType {
    OneOrMore,
    One,
    OneOf(HashSet<char>),
    NotOneOf(HashSet<char>),
    Literal(char),
}

pub struct Pattern {
    pattern: Vec<PatternType>,
    pub inclusive: bool,
}

impl Pattern {
    pub fn parse(pattern: &str, is_inclusive: bool) -> Self {
        let mut mapped_pattern: Vec<PatternType> = vec![];
        let mut chars_iter = pattern.chars().peekable();

        let mut active_group: Option<Vec<char>> = None;
        let mut is_exclusive: bool = false;

        while let Some(ch) = chars_iter.next() {
            match ch {
                '*' => mapped_pattern.push(PatternType::OneOrMore),
                '?' => mapped_pattern.push(PatternType::One),
                '[' if !active_group.is_some() => active_group = Some(vec![]),
                ']' => {
                    let char_set = active_group
                        .take()
                        .unwrap()
                        .into_iter()
                        .collect::<HashSet<_>>();

                    let pattern_type = if is_exclusive {
                        is_exclusive = false;
                        PatternType::NotOneOf(char_set)
                    } else {
                        PatternType::OneOf(char_set)
                    };

                    mapped_pattern.push(pattern_type);
                }
                '-' if active_group.is_some() => {
                    let mut group = active_group.take().unwrap();
                    let previous_char =
                        group.pop().expect("Invalid wildcard pattern");

                    // This is a range in a bracketed group
                    if previous_char.is_ascii_alphanumeric() {
                        let end_range_val = chars_iter
                            .next()
                            .expect("Invalid wildcard pattern");

                        let mut all_vals = (previous_char as u32
                            ..=end_range_val as u32)
                            .filter_map(std::char::from_u32)
                            .collect::<Vec<char>>();

                        group.append(&mut all_vals);
                    } else {
                        // This is a hyphen literal in a bracketed group
                        group.push(ch)
                    }
                    active_group = Some(group);
                }
                '!' if chars_iter.peek().is_some()
                    && *chars_iter.peek().unwrap() == '[' =>
                {
                    is_exclusive = true
                }
                '|' if active_group.is_some() => (),
                _ => {
                    if active_group.is_some() {
                        let mut current = active_group.unwrap();
                        current.push(ch);
                        active_group = Some(current);
                    } else {
                        mapped_pattern.push(PatternType::Literal(ch));
                    }
                }
            }
        }

        Self {
            pattern: mapped_pattern,
            inclusive: is_inclusive,
        }
    }

    pub fn match_single(ch: &char, pattern: &PatternType) -> bool {
        match pattern {
            PatternType::OneOf(char_set) => char_set.contains(ch),
            PatternType::NotOneOf(char_set) => !char_set.contains(ch),
            PatternType::Literal(pat_char) => pat_char == ch,
            _ => true, // ? (any single char) or * (not followed by additional pattern)
        }
    }

    pub fn is_match(&self, value: &str) -> bool {
        let mut pattern_iter = self.pattern.iter().peekable();
        let mut val_chars = value.chars();

        let mut is_match = true;

        while let Some(pattern) = pattern_iter.next() {
            let ch_idx = val_chars.next();

            if ch_idx.is_none() {
                is_match = false;
                break;
            }

            let ch = ch_idx.unwrap();

            match pattern {
                // OneOrMore needs special handling only when it's followed by
                // additional pattern matching characters. When this is the case,
                // it needs to eat until a char matches the next pattern
                // or we exhaust the chars iterator.
                PatternType::OneOrMore if pattern_iter.peek().is_some() => {
                    let next_pattern = pattern_iter.next().unwrap();
                    let mut next_match = false;

                    for ch in val_chars.by_ref() {
                        if Self::match_single(&ch, next_pattern) {
                            next_match = true;
                            break;
                        }
                    }

                    // exhausted the chars iter without finding a match
                    if !next_match {
                        is_match = false
                    }
                }
                _ => is_match = Self::match_single(&ch, pattern),
            }

            if !is_match {
                break;
            }
        }

        is_match
    }
}

#[cfg(test)]
mod pattern_parsing_tests {
    use super::*;

    #[test]
    fn parses_asterisk_pattern_base() {
        let result = Pattern::parse("*", true);
        assert_eq!(result.pattern, vec![PatternType::OneOrMore])
    }

    #[test]
    fn parses_question_mark_pattern_base() {
        let result = Pattern::parse("?", true);
        assert_eq!(result.pattern, vec![PatternType::One])
    }

    #[test]
    fn parses_enumerated_bracket_set_base() {
        let result = Pattern::parse("[abcde]", true);
        assert_eq!(
            result.pattern,
            vec![PatternType::OneOf(HashSet::from(['a', 'b', 'c', 'd', 'e']))]
        )
    }

    #[test]
    fn parses_hyphenated_bracket_range_base() {
        let result = Pattern::parse("[a-c]", true);
        assert_eq!(
            result.pattern,
            vec![PatternType::OneOf(HashSet::from(['a', 'b', 'c']))]
        )
    }

    #[test]
    fn parses_multi_hyphenated_bracket_ranges() {
        let result = Pattern::parse("[a-cD-F0-5]", true);
        assert_eq!(
            result.pattern,
            vec![PatternType::OneOf(HashSet::from([
                'a', 'b', 'c', 'D', 'E', 'F', '0', '1', '2', '3', '4', '5'
            ]))]
        )
    }

    #[test]
    fn parses_combined_patterns() {
        let result = Pattern::parse("ctx-[a-c]??_t*", true);
        assert_eq!(
            result.pattern,
            vec![
                PatternType::Literal('c'),
                PatternType::Literal('t'),
                PatternType::Literal('x'),
                PatternType::Literal('-'),
                PatternType::OneOf(HashSet::from(['a', 'b', 'c'])),
                PatternType::One,
                PatternType::One,
                PatternType::Literal('_'),
                PatternType::Literal('t'),
                PatternType::OneOrMore
            ]
        )
    }
}

#[cfg(test)]
mod pattern_matching_tests {
    use super::*;

    #[test]
    fn one_or_more() {
        let pattern = Pattern::parse("*", true);

        let is_match = pattern.is_match("abc");
        assert!(is_match == true);
    }

    #[test]
    fn one_or_more_miss() {
        let pattern = Pattern::parse("abc*", true);

        let is_match = pattern.is_match("abc");
        assert!(is_match == false);
    }

    #[test]
    fn one_or_more_surrounded_by_literals() {
        let pattern = Pattern::parse("a*c", true);

        let is_match = pattern.is_match("a_b_l_j_k_c");
        assert!(is_match == true);
    }

    #[test]
    fn one_or_more_final_pattern() {
        let pattern = Pattern::parse("a_b*", true);

        let is_match = pattern.is_match("a_b_l_j_k_c");
        assert!(is_match == true);
    }

    #[test]
    fn inclusive_bracket_match_enumerated() {
        let pattern = Pattern::parse("a[bljk_]c", true);

        let is_match = pattern.is_match("a_c");
        assert!(is_match == true);
    }

    #[test]
    fn inclusive_bracket_match_enumerated_hypen_literal() {
        let pattern = Pattern::parse("a[bljk_-]c", true);

        let is_match = pattern.is_match("a-c");
        assert!(is_match == true);
    }

    #[test]
    fn inclusive_bracket_match_range() {
        let pattern = Pattern::parse("a[b-k]c", true);

        let is_match = pattern.is_match("ajc");
        assert!(is_match == true);
    }

    #[test]
    fn inclusive_bracket_match_multi_range() {
        let pattern = Pattern::parse("a[b-k|0-9]c", true);

        let is_match = pattern.is_match("a7c");
        assert!(is_match == true);
    }

    #[test]
    fn inclusive_bracket_match_range_miss() {
        let pattern = Pattern::parse("a[b-k]c", true);

        let is_match = pattern.is_match("alc");
        assert!(is_match == false);
    }

    #[test]
    fn exclusive_bracket_match_enumerated_miss() {
        let pattern = Pattern::parse("a![bljk_]c", true);

        let is_match = pattern.is_match("a_c");
        assert!(is_match == false);
    }

    #[test]
    fn exclusive_bracket_match_range_miss() {
        let pattern = Pattern::parse("a![b-k]c", true);

        let is_match = pattern.is_match("ajc");
        assert!(is_match == false);
    }

    #[test]
    fn exclusive_bracket_match_range() {
        let pattern = Pattern::parse("a![b-k]c", true);

        let is_match = pattern.is_match("alc");
        assert!(is_match == true);
    }

    #[test]
    fn combined_pattern_match() {
        let pattern = Pattern::parse("ctx-[a-c]??_t*", true);

        let is_match = pattern.is_match("ctx-bcc_trest");
        assert!(is_match == true);
    }

    #[test]
    fn combined_pattern_match_1() {
        let pattern = Pattern::parse("ctx-*-[a-c]??_t*", true);

        let is_match = pattern.is_match("ctx-qrs-bcc_trest");
        assert!(is_match == true);
    }

    #[test]
    fn combined_pattern_match_miss() {
        let pattern = Pattern::parse("ctx-*-[a-c]??_t*", true);

        let is_match = pattern.is_match("ctx-qrsbcc_trest-");
        assert!(is_match == false);
    }

    #[test]
    fn combined_pattern_miss() {
        let pattern = Pattern::parse("ctx-[a-c]??_t*", true);

        let is_match = pattern.is_match("ctx-bcc_t");
        assert!(is_match == false);
    }
}
