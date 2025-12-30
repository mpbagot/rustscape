use fuzzbunny_rs::fuzzy_match;

fn check_highlights(target: &str, search: &str, expected: Vec<&str>) {
  let highlights = fuzzy_match(Some(target), Some(search)).unwrap().highlights;
  assert_eq!(highlights, expected);
}

#[test]
fn matches_string_start() {
  check_highlights("abcdefg", "abc", vec!["", "abc", "defg"]);
}

#[test]
fn matches_string_middle() {
  check_highlights("abcdefg", "def", vec!["abc", "def", "g"]);
  check_highlights("abcdefg", "efg", vec!["abcd", "efg"]);
}

#[test]
fn matches_none() {
  assert!(fuzzy_match(Some("abcdefg"), Some("zx")).is_none());
}

#[test]
fn matches_prefix_filter() {
  check_highlights("ab cdefg", "ac", vec!["", "a", "b ", "c", "defg"]);
}

#[test]
fn matches_case_insensitive() {
  check_highlights("abcdefg", "dEf", vec!["abc", "def", "g"]);
  check_highlights("abCDEfg", "dEF", vec!["abC", "DEf", "g"]);
}

#[test]
fn matches_ignores_whitespace() {
  check_highlights("abcdefg", "   def", vec!["abc", "def", "g"]);
  check_highlights("abcdefg", "abc   ", vec!["", "abc", "defg"]);
  check_highlights("abcdefg", "  abc ", vec!["", "abc", "defg"]);
}

#[test]
fn matches_search_substring() {
  check_highlights("This is a test", "this is", vec!["", "This is", " a test"]);

  assert!(fuzzy_match(Some("This should not match"), Some("this is")).is_none());
}

#[test]
fn matches_no_filter() {
  check_highlights("abcdefg", "", vec!["abcdefg"]);

  let highlights = fuzzy_match(Some("abcdefg"), None).unwrap().highlights;
  assert_eq!(highlights, vec!["abcdefg"]);
}

#[test]
fn matches_contiguous() {
  check_highlights("abcd efg", "bcd efg", vec!["a", "bcd efg"]);
}

#[test]
fn matches_separated_fails() {
  assert!(fuzzy_match(Some("abcdefg"), Some("abc xxx")).is_none());
}

#[test]
fn matches_quotes_substrings() {
  check_highlights("a b c abC def", "abc d", vec!["a b c ", "abC d", "ef"]);
  check_highlights("Las Vegas", "\"la", vec!["", "La", "s Vegas"]);

  assert!(fuzzy_match(Some("a bc def"), Some("\"abc d\"")).is_none());
  assert!(fuzzy_match(Some("Los Angeles"), Some("\"LA")).is_none());
}

#[test]
fn matches_normal_with_quotes_in_middle() {
  check_highlights("abc \"def\"", "a\"def\"", vec!["", "a", "bc ", "\"def\""]);

  assert!(fuzzy_match(Some("Las Vegas"), Some("la\"")).is_none());
}

#[test]
fn matches_camel_title_initials() {
  check_highlights("FuzzBunny", "fb", vec!["", "F", "uzz", "B", "unny"]);
  check_highlights("fuzzBunny.ts", "fb", vec!["", "f", "uzz", "B", "unny.ts"]);
  check_highlights("fuzzBunnyIsAwesome", "bia", vec!["fuzz", "B", "unny", "I", "s", "A", "wesome"]);
}
