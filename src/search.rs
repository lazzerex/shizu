pub fn find_all(text: &str, query: &str) -> Vec<usize> {
    let haystack: Vec<char> = text.chars().collect();
    let needle: Vec<char> = query.chars().collect();
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    haystack
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle.as_slice())
        .map(|(start, _)| start)
        .collect()
}

pub fn next_match(matches: &[usize], from: usize) -> Option<usize> {
    matches
        .iter()
        .copied()
        .find(|&start| start >= from)
        .or_else(|| matches.first().copied())
}

pub fn previous_match(matches: &[usize], before: usize) -> Option<usize> {
    matches
        .iter()
        .copied()
        .rev()
        .find(|&start| start < before)
        .or_else(|| matches.last().copied())
}

#[cfg(test)]
mod tests {
    use super::{find_all, next_match, previous_match};

    #[test]
    fn finds_every_occurrence() {
        assert_eq!(find_all("abcabc", "abc"), vec![0, 3]);
    }

    #[test]
    fn no_matches_is_empty() {
        assert!(find_all("abc", "xyz").is_empty());
        assert!(find_all("abc", "").is_empty());
        assert!(find_all("", "abc").is_empty());
    }

    #[test]
    fn overlapping_matches_are_reported() {
        assert_eq!(find_all("aaaa", "aa"), vec![0, 1, 2]);
    }

    #[test]
    fn match_across_line_boundary() {
        assert_eq!(find_all("ab\ncd", "b\nc"), vec![1]);
    }

    #[test]
    fn char_indices_not_byte_indices() {
        assert_eq!(find_all("héllo hé", "hé"), vec![0, 6]);
    }

    #[test]
    fn next_wraps_to_first() {
        let matches = [2, 5, 9];
        assert_eq!(next_match(&matches, 3), Some(5));
        assert_eq!(next_match(&matches, 9), Some(9));
        assert_eq!(next_match(&matches, 10), Some(2));
        assert_eq!(next_match(&[], 0), None);
    }

    #[test]
    fn previous_wraps_to_last() {
        let matches = [2, 5, 9];
        assert_eq!(previous_match(&matches, 6), Some(5));
        assert_eq!(previous_match(&matches, 2), Some(9));
        assert_eq!(previous_match(&matches, 0), Some(9));
        assert_eq!(previous_match(&[], 0), None);
    }
}
