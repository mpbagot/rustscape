//! A Rust reimplementation of the original [fuzzbunny](https://github.com/mixpanel/fuzzbunny/) JS library.
//!
//! This implementation provides a similar API as the original, with various tweaks to better suit
//! the specific requirements of the parent Rustscape project.
//!
//! ## Features
//!
//! - **Fuzzy matching**: Perform efficient fuzzy string matching based on string prefixes
//! - **Parallel processing**: Leverages `rayon` for parallelized filtering and sorting
//! - **Highlighting**: Automatically generates highlighted substrings for matched ranges
//! - **Performance optimizations**: Uses precomputed skip indices for efficient prefix matching
//!
//! ## Usage
//!
//! ```rust
//! use fuzzbunny_rs::{fuzzy_filter, precompute_skips_for_items};
//!
//! let items = vec!["apple", "application", "banana"];
//! let targets = precompute_skips_for_items(items);
//! let results = fuzzy_filter(&targets, "app");
//!
//! assert_eq!(*results[0].highlights.as_ref().unwrap(), vec!["", "app", "le"]);
//! assert_eq!(*results[1].highlights.as_ref().unwrap(), vec!["", "app", "lication"]);
//! assert_eq!(results.len(), 2);
//! ```
//!
//! ## Scoring
//!
//! The scoring algorithm rewards:
//! - Matches at the beginning of strings
//! - Contiguous matches (longer matches score higher)
//! - Matches closer to the start of the string

use rayon::prelude::*;

const SCORE_START_STR: u32 = 1000;
const SCORE_PREFIX: u32 = 200;
const SCORE_CONTIGUOUS: u32 = 300;

/// Highlighted substrings of a full string.
///
/// Every second string in the [`Vec`] represents a substring that matches
/// with the search string.
///
/// # Examples
///
/// 'usam' matches the string: 'the \[u\]nited \[s\]tates of \[am\]erica'.
/// The highlights for such a match would be:
/// ```
/// ["the ", "u", "nited ", "s", "tates of ", "am", "erica"];
/// ```
/// Which could be rendered as:
///
/// 'the **u**nited **s**tates of **am**erica'
pub type Highlights<'a> = Vec<&'a str>;

/// A target string to fuzzy search within.
///
/// Optionally includes a skip index vector. If included, these skip indices
/// are used during processing to reduce repeated calculation.
pub type Target<'a> = (&'a str, Option<Vec<usize>>);

/// Match score and ranges on a string.
///
/// The `score` represents the match score for the string, while `ranges` holds
/// [`Range`] items for each substring that matches between a search and target string.
pub struct StringScore {
    /// The match score for a search string against a target string.
    pub score: u32,
    /// The ranges in the target string that matches with the search string.
    pub ranges: Vec<Range>,
}

/// A matched substring range in a larger string.
#[derive(Debug)]
pub struct Range(
    /// The start index of the match range.
    pub usize,
    /// The length of the match range.
    pub usize,
);

impl Range {
    /// Calculate the byte index of the final character of the range.
    #[inline]
    const fn end_index(&self) -> usize {
        self.0 + self.1
    }

    /// Merge another [`Range`] into this one by concatenation.
    ///
    /// # Panics
    ///
    /// This function panics if this range doesn't directly precede the one to be merged.
    #[inline]
    fn merge(&mut self, other: Range) {
        assert_eq!(self.end_index(), other.0);
        self.1 += other.1;
    }

    /// Calculate a match score for this range.
    ///
    /// Score increases exponentially for contiguous matches, and are generally higher
    /// for matches closer to the beginning of the string.
    #[inline]
    const fn get_score(&self, is_prefix: bool) -> u32 {
        let mut score: u32 = 0;

        // increase score exponentially per letter matched so that contiguous matches are ranked higher
        // i.e '[abc]' ranks higher than '[ab]ott [c]hemicals'
        score += SCORE_CONTIGUOUS * ((self.1 * self.1) as u32); // u16 * u16 can at most be u32

        score += if self.0 == 0 {
            // matching at the start of string gets a ranking bonus
            SCORE_START_STR
        } else if is_prefix {
            // closer to the start, the higher it ranks
            SCORE_PREFIX - self.0 as u32 // We assume the input string won't be more than u32::MAX in length
        } else {
            0
        };

        score
    }
}

/// Filter result for a target string including match score and highlights.
#[derive(Debug)]
pub struct FuzzyFilterResult<'a> {
    /// The target string that the search string was matched against.
    pub item: &'a str,
    /// The match score for a search string against a target string.
    pub score: u32,
    /// The highlight substrings of the target string. See [`Highlights`]. [`None`] if there is no match.
    pub highlights: Option<Highlights<'a>>,
}

impl<'a> PartialEq for FuzzyFilterResult<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.item == other.item
    }
}
impl<'a> Eq for FuzzyFilterResult<'a> {}
impl<'a> PartialOrd for FuzzyFilterResult<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self.score, &other.score)
            .or_else(|| PartialOrd::partial_cmp(&other.item, &self.item))
    }
}
impl<'a> Ord for FuzzyFilterResult<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.score, &other.score)
            .then_with(|| Ord::cmp(&other.item, &self.item))
    }
}

/// Convert an interator of string items to a [`Target`] vector.
///
/// This is a convenience function to quickly convert a set of plain strings
/// into [`Target`] items for use with [`fuzzy_filter`]. If you don't want the skips
/// computed, you can manually wrap string items as `(item, None)` instead.
///
/// # Returns
///
/// The string items wrapped as [`Target`] items including the precomputed skip indices.
pub fn precompute_skips_for_items<'a>(items: impl IntoIterator<Item = &'a str>) -> Vec<Target<'a>> {
    items
        .into_iter()
        .map(|string| (string, Some(get_target_skips(string))))
        .collect()
}

/// Perform a prefix match for a search string on the target string.
///
/// This function starts from the given skip, and checks against all skip indices from then on.
/// For example:
/// 'usam' matches '[u]nited [s]tates of [am]erica', with skip index 0.
/// 'usam' matches 'the [u]nited [s]tates of [am]erica', with skip index 1, but not 0.
///
/// # Returns
///
/// [`None`] if the search string doesn't match word prefixes starting at the given index.
/// Otherwise, returns the match [`Range`]s as a [`Vec`].
#[inline]
fn fuzzy_prefix_match(skip_idx: usize, search: &str, target: &str, target_skips: &Vec<usize>) -> Option<Vec<Range>> {
    let mut ranges: Vec<Range> = Vec::with_capacity(target_skips.len());
    let mut search_chars = search.bytes();
    let mut search_char = search_chars.next();

    for i in skip_idx..target_skips.len() - 1 {
        let start_idx = target_skips[i];
        let mut target_cnt = target_skips[i + 1] - start_idx;
        let mut match_len = 0;

        // Set up character iterators
        let mut target_chars = target.bytes();

        // Initialise the characters to the start of the ranges
        let mut target_char = target_chars.nth(start_idx);

        while target_char.is_some() && search_char.is_some() && target_cnt > 0 {
            // Safe to unwrap
            let t_char = target_char.unwrap();
            let s_char = search_char.unwrap();

            if t_char == s_char {
                target_char = target_chars.next();
                search_char = search_chars.next();
                match_len += 1;
                target_cnt -= 1; // Decrement when target_char increments
                continue;
            }

            // spaces shouldn't break matching
            if t_char == b' ' {
                target_char = target_chars.next();
                target_cnt -= 1; // Decrement when target_char increments
                continue;
            }
            if s_char == b' ' {
                search_char = search_chars.next();
                continue;
            }

            break;
        }

        // Make contiguous ranges if possible
        // TODO This looks terrible
        if match_len > 0 {
            let this_range = Range(start_idx, match_len);
            // If the most recent range butts up against this one, simply extend the previous
            if ranges.len() == 0 {
                ranges.push(this_range);
            } else {
                let prev_range = ranges.last_mut().unwrap();
                if prev_range.end_index() == start_idx {
                    // Update previous range
                    prev_range.merge(this_range)
                } else {
                    // Add restore the previous range and add the new this_range
                    ranges.push(this_range);
                };
            }
        }

        if search_char.is_none() {
            // Search is fully matched, return ranges
            return Some(ranges)
        }
    }

    None
}

/// Compute skip indices for a target string.
///
/// Skip indices mark word and punctuation boundaries, including camel/PascalCase
/// case changes. These are used to quickly find prefix matches in the target string
/// without traversing the entire string each time.
#[inline]
pub fn get_target_skips(target: &str) -> Vec<usize> {
    let mut target_skips = vec![];
    let mut was_alpha_num = false;
    let mut was_upper_case = false;
    let mut i = 0;

    for char in target.chars() {
        let is_alpha_num = char.is_alphanumeric();
        let is_upper_case = char.is_uppercase();

        if (is_alpha_num && !was_alpha_num) || (is_upper_case && !was_upper_case) || char.is_ascii_punctuation() {
            target_skips.push(i);
        }

        was_alpha_num = is_alpha_num;
        was_upper_case = is_upper_case;
        i += 1;
    }

    // We push the length as the last skip so when matching
    // every range aligns between skip[i] and skip[i + 1]
    // and we don't have to do extraneous overflow checks
    target_skips.push(target.len());

    // NOTE: these can possibly be cached on the items for a faster search next time
    target_skips
}

/// Calculate the highlighted substrings of a target string for the given match ranges.
///
/// This function converts the `ranges` of a [`StringScore`] into the equivalent [`Highlights`]
/// that you would find in a [`FuzzyFilterResult`].
///
/// # Examples
///
/// ```rust
/// use fuzzbunny_rs::{highlights_from_ranges, Range};
///
/// let hls = highlights_from_ranges("my example", vec![Range(3, 2)]);
/// assert_eq!(hls, vec!["my ", "ex", "ample"]);
/// ```
#[inline]
pub fn highlights_from_ranges<'a>(target: &'a str, ranges: Vec<Range>) -> Highlights<'a> {
    let mut last_index = 0;
    let mut highlights = Vec::with_capacity(ranges.len() * 2 + 1);

    for range in ranges {
        let start_index = range.0;
        let end_index = range.end_index();
        highlights.push(&target[last_index..start_index]);
        highlights.push(&target[start_index..end_index]);
        last_index = end_index;
    }

    if last_index < target.len() {
        highlights.push(&target[last_index..]);
    }

    highlights
}

/// Compute a raw score and highlight ranges for a target and search string.
///
/// This is a slightly lower level call. If performance is of importance and you want to avoid
/// trim + highlighting on every item, use this and only call [`highlights_from_ranges`]
/// for only the items that need the highlights.
///
/// Note that `search` string MUST be lower case.
pub fn fuzzy_score_item(target: &Target<'_>, search: &str) -> Option<StringScore> {
    if target.0.len() == 0 {
        return None
    }

    // empty search string is technically a match of nothing
    if search.len() == 0 {
        return Some(StringScore { score: 0, ranges: vec![] })
    }

    let mut search_str = search;

    // if user enters a quoted search then only perform substring match
    // e.g "la matches [{La}s Vegas] but not [Los Angeles]
    // NOTE: ending quote is optional so user can get incremental matching as they type.
    let is_quoted_search_str = search.bytes().next().is_some_and(|char| char == b'"');
    if is_quoted_search_str {
        let end_index = if search.ends_with('"') { search.len() - 1 } else { search.len() };
        search_str = &search[1..end_index];
    }


    // try substring search first
    let l_case_target_str = target.0.to_lowercase();
    let match_idx = l_case_target_str.find(search_str);
    let search_len = search_str.len();

    if match_idx.is_some() {
        let idx = match_idx.unwrap();
        let match_range = Range(idx, search_len);
        let is_word_prefix = idx > 0 && !char::from(target.0.bytes().nth(idx - 1).unwrap()).is_alphanumeric();
        return Some(StringScore {
            score: match_range.get_score(is_word_prefix),
            ranges: vec![match_range]
        })
    }

    // if we didn't match a single character as a substr, we won't fuzzy match it either, exit early.
    // if quoted search, exit after substring search as well, since user doesn't want fuzzy search.
    if search_len == 1 || is_quoted_search_str {
        return None
    }

    // fall back to fuzzy matching which matches word prefixes or punctuations
    // because we've precomputed targetSkips, its O(m+n) for avg case
    // the skip array helps us make faster alignments, rather than letter by letter
    let target_skips = if target.1.is_none() {
        &get_target_skips(target.0)
    } else {
        target.1.as_ref().unwrap()
    };

    let first_search_char = search_str.bytes().next().unwrap();
    for skip_idx in 0..(target_skips.len() - 1) {
        let tgt_idx = target_skips[skip_idx];
        let targ_char = l_case_target_str.bytes().nth(tgt_idx).unwrap();
        if targ_char == first_search_char {
            // possible alignment, perform prefix match
            let ranges = fuzzy_prefix_match(skip_idx, search, &l_case_target_str, &target_skips);
            if ranges.is_some() {
                let ranges = ranges.unwrap();
                let score = ranges.iter().map(|rng| rng.get_score(true)).sum();
                return Some(StringScore { score, ranges })
            }
        }
    }

    None
}

/// Fuzzy match a target string with a search string.
///
/// # Returns
///
/// A [`FuzzyFilterResult`] holding the target string, score and highlighted substring sections
/// if the search string fuzzily matches inside the target. [`None`] otherwise
pub fn fuzzy_match<'t>(target: &'t str, search: Option<&str>) -> Option<FuzzyFilterResult<'t>> {
    let search: &str = &search.unwrap_or("").trim().to_lowercase();

    let string_match = fuzzy_score_item(&(target, None), search);

    string_match.map(|mat| {
        FuzzyFilterResult {
            item: target,
            score: mat.score,
            highlights: Some(highlights_from_ranges(target, mat.ranges)),
        }
    })
}

/// Search a vector of [`Target`]s and return a filtered and sorted vector
/// of [`FuzzyFilterResult`].
///
/// Each provided target is scored against the `search` string. Only non-zero scores are returned.
///
/// This version makes use of rayon to parallelise the scoring (an embarrassingly parallel problem)
/// and sorting the scored results.
pub fn fuzzy_filter<'a>(items: &Vec<Target<'a>>, search: &str) -> Vec<FuzzyFilterResult<'a>> {
    let search_lower_cased = search.trim().to_lowercase();

    // In parallel, process the results
    let mut results: Vec<FuzzyFilterResult<'a>> = items
        .into_par_iter()
        .map(|target| {
            let match_item = fuzzy_score_item(target, &search_lower_cased);
            match_item.map_or(
                FuzzyFilterResult { item: target.0, score: 0, highlights: None },
                |match_item| {
                    FuzzyFilterResult {
                        item: target.0,
                        score: match_item.score,
                        highlights: Some(highlights_from_ranges(target.0, match_item.ranges)),
                    }
                }
            )
        })
        .filter(|res| res.highlights.is_some())
        .collect();

    if search.len() > 0 {
        // Then sort in parallel.
        results.par_sort_by(|a, b| b.cmp(a));
    }

    results
}
