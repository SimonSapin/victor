use victor::text::*;

static ALICE: &'static str = include_str!("alice.txt");

#[test]
fn breaks() {
    let words = split_at_breaks(ALICE);
    assert_eq!(&words[..10], &["CHAPTER ", "II. ", "The ", "Pool ", "of ", "Tears\n", "\n",
                              "‘Curiouser ", "and ", "curiouser!’ "]);
}

#[test]
fn hard_breaks() {
    let lines = split_at_hard_breaks(ALICE);
    assert_eq!(lines[0], "CHAPTER II. The Pool of Tears\n");
    assert_eq!(lines[1], "\n");
    assert!(lines.last().unwrap().ends_with("I am so VERY tired of being all alone here!’\n"))
}
