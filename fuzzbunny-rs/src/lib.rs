//! TODO Crate level docs here

use rayon::prelude::*;

const SCORE_START_STR: u32 = 1000;
const SCORE_PREFIX: u32 = 200;
const SCORE_CONTIGUOUS: u32 = 300;

/// TODO
pub type Highlights<'a> = Vec<&'a str>;

/// TODO
pub type Target<'a> = (&'a str, Option<Vec<usize>>);

/// TODO
pub struct StringScore {
    pub score: u32,
    pub ranges: Vec<Range>
}

/// TODO A range inside a string
#[derive(Debug)]
pub struct Range(usize, usize);

impl Range {
    /// TODO
    #[inline]
    const fn end_index(&self) -> usize {
        self.0 + self.1
    }

    /// TODO
    #[inline]
    fn merge(&mut self, other: Range) {
        assert_eq!(self.end_index(), other.0);
        self.1 += other.1;
    }

    /// TODO
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

/// TODO
#[derive(Debug)]
pub struct FuzzyFilterResult<'a> {
    pub item: &'a str,
    pub score: u32,
    pub highlights: Option<Highlights<'a>>
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

/// TODO
pub fn precompute_skips_for_items<'a>(items: impl IntoIterator<Item = &'a str>) -> Vec<Target<'a>> {
    items
        .into_iter()
        .map(|string| (string, Some(get_target_skips(string))))
        .collect()
}

/// Perform a prefix match for a search string on the target string.
///
// /**
//  * performs a prefix match e.g 'usam' matches '[u]nited [s]tates of [am]erica
//  * @param {number} skipIdx - skip index where to start search from
//  * @param {string} searchStr - lowercased search string
//  * @param {string} targetStr - lowercased target string
//  * @param {number[]} targetSkips - skip boundary indices
//  * @returns {number[] | null}
//  *  - the [idx, len, ...] ranges where the match occured
//  *  - null if no match found
//  */
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

/// TODO
///
/// /**
//  * A skip index marks word and punctuation boundaries
//  * We use this to skip around the targetStr and quickly find prefix matches
//  * @param {string} targetStr
//  * @returns {number[]}
//  */
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

/// Returns the string parts for highlighting from the matched ranges
///
/// TODO Example ('my example', [3, 2]) would return ['my ', 'ex', 'ample']
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

/// TODO
///
/// /**
//  * fuzzyScoreItem is called by fuzzyMatch, it's a slightly lower level call
//  * If perf is of importance and you want to avoid lowercase + trim + highlighting on every item
//  * Use this and only call highlightsFromRanges for only the items that are displayed
//  * @param {string} targetStr - lowercased trimmed target string to search on
//  * @param {string} searchStr - lowercased trimmed search string
//  * @returns {{score: number, ranges: number[]} | null} - null if no match
//  */
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

/// Fuzzy match and return the score, highlights, and lowercased matchStr (for sort)
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

/// TODO
// /**
//  * Searches an array of items on props and returns filtered + sorted array with scores and highlights
//  * @template Item
//  * @param {Item[]} items
//  * @param {string} searchStr
//  * @param {{fields: (keyof Item)[]}} options
//  * @returns {FuzzyFilterResult<Item>[]}
//  */
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
