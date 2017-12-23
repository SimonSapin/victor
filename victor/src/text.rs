use xi_unicode::LineBreakIterator;

pub fn split_at_breaks(s: &str) -> Vec<&str> {
    let mut last_break = 0;
    LineBreakIterator::new(s).map(|(position, _)| {
        let range = last_break..position;
        last_break = position;
        &s[range]
    }).collect()
}

pub fn split_at_hard_breaks(s: &str) -> Vec<&str> {
    let mut last_break = 0;
    LineBreakIterator::new(s).filter(|&(_, is_hard_break)| is_hard_break).map(|(position, _)| {
        let range = last_break..position;
        last_break = position;
        &s[range]
    }).collect()
}
