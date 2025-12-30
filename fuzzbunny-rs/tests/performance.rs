use fuzzbunny_rs::{fuzzy_filter, get_target_skips};
use std::fs::File;
use std::io::{self, BufRead};

#[test]
fn fuzzy_score_item_bench() {
  let file = File::open("tests/gutenberg-catalog.txt").unwrap();
  let lines: Result<Vec<String>, _> = io::BufReader::new(file).lines().skip(1).collect();
  let lines = lines.unwrap();
  let line_count = lines.len();
  let ref_lines = (0..line_count).map(|i| {
    let tgt = lines[i].as_str();
    let skips = Some(get_target_skips(tgt));
    (tgt, skips)
  }).collect();

  let lines_per_sec_low_bar = 500_000 as f64;
  let words = ["oliver", "alice", "mayflo", "declofusa", "audio"];
  let start_time = std::time::Instant::now();

  for word in words {
    fuzzy_filter(&ref_lines, word);
  }

  let elapsed_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
  let num_lines_matched = (line_count * words.len() * 1000) as f64;
  let lines_per_sec = num_lines_matched / elapsed_time_ms;

  std::println!("matched {} lines/sec", lines_per_sec);
  assert!(lines_per_sec > lines_per_sec_low_bar);
}