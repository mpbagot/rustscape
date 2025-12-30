use fuzzbunny_rs::{Highlights, fuzzy_filter, precompute_skips_for_items};

// from https://en.wikipedia.org/wiki/List_of_Heroes_characters#Main_characters
const HEROES_CSV: &'static str = "Claire Bennet, Rapid cellular regeneration
Elle Bishop, Electrokinesis
Monica Dawson, Adaptive muscle memory
EL Hawkins, Phasing
Maya Herrera, Poison emission
Isaac Mendez, Precognition
Adam Monroe, Immortality
Hiro Nakamura, Space-time manipulation
Matt Parkman, Telepathy
Angela Petrelli, Enhanced dreaming
Nathan Petrelli, Flight
Peterr Petrelli, Empathic mimicry then tactile power mimicry
Arthur Petrelli, Ability absorption
Micah Sanders, Technopathy
Niki Sanders, Enhanced strength
Tracy Strauss, Cryokinesis
Samuel Sullivan, Terrakinesis
Gabriel Gray / Sylar, Power mimicry and amplification";

fn make_heroes() -> Vec<&'static str> {
    HEROES_CSV.trim().split('\n').collect()
}

fn get_highlights(search: &str) -> Vec<Highlights<'static>>{
    let heroes = precompute_skips_for_items(make_heroes());
    let results = fuzzy_filter(&heroes, search);
    results
        .into_iter()
        .map(|res| res.highlights)
        .filter(|opt| opt.is_some())
        .map(|opt| opt.unwrap())
        .collect()
}

#[test]
fn filter_preserve_order_empty_search() {
    let highlights = get_highlights("");
    let expected: Vec<Highlights<'static>> = make_heroes()
        .into_iter()
        .map(|hero| vec![hero])
        .collect();
    assert_eq!(highlights, expected);
}

#[test]
fn filter_matches_string_beginning() {
    let highlights = get_highlights("TE");
    let expected = vec![
        vec!["Matt Parkman, ", "Te", "lepathy"],
        vec!["Micah Sanders, ", "Te", "chnopathy"],
        vec!["Samuel Sullivan, ", "Te", "rrakinesis"],
        vec!["Pe", "te", "rr Petrelli, Empathic mimicry then tactile power mimicry"],
    ];
    assert_eq!(highlights, expected);
}

#[test]
fn filter_matches_string_middle() {
    let highlights = get_highlights("mimi");
    let expected = vec![
        vec!["Peterr Petrelli, Empathic ", "mimi", "cry then tactile power mimicry"],
        vec!["Gabriel Gray / Sylar, Power ", "mimi", "cry and amplification"],
    ];
    assert_eq!(highlights, expected);
}

#[test]
fn filter_matches_string_exact() {
    let highlights = get_highlights("petrelli");
    let expected = vec![
        vec!["Angela ", "Petrelli", ", Enhanced dreaming"],
        vec!["Arthur ", "Petrelli", ", Ability absorption"],
        vec!["Nathan ", "Petrelli", ", Flight"],
        vec!["Peterr ", "Petrelli", ", Empathic mimicry then tactile power mimicry"],
    ];
    assert_eq!(highlights, expected);
}
