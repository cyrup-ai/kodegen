// Tests extracted from src/core/tokenizer/core.rs

use kodegen_candle_agent::core::tokenizer::core::{CandleTokenizerConfig, utils};

#[test]
fn test_tokenizer_config_default() {
    let config = CandleTokenizerConfig::default();
    assert_eq!(config.max_length, 4096);
    assert!(config.add_special_tokens);
    assert_eq!(config.eos_token_id, Some(2));
}

#[test]
fn test_padding_strategy() {
    let _sequences = [vec![1, 2, 3], vec![1, 2, 3, 4, 5], vec![1]];

    // This test would require a real tokenizer instance
    // let tokenizer = create_test_tokenizer();
    // tokenizer.pad_sequences(&mut sequences);
    // assert_eq!(sequences[0].len(), 5);
    // assert_eq!(sequences[2].len(), 5);
}

#[test]
fn test_token_overlap_calculation() {
    let seq1 = vec![1, 2, 3, 4, 5];
    let seq2 = vec![4, 5, 6, 7, 8];

    let overlap = utils::calculate_overlap(&seq1, &seq2);
    assert_eq!(overlap, 2);

    let merged = utils::merge_sequences(&seq1, &seq2, overlap);
    assert_eq!(merged, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn test_token_count_estimation() {
    let text = "Hello world this is a test";
    let estimated = utils::estimate_token_count(text);
    assert!(estimated > 0);
    assert!(estimated <= text.len()); // Should be reasonable estimate
}

#[test]
fn test_text_splitting() {
    let text = "Hello world this is a very long sentence that needs to be split";
    let (part1, part2) = utils::split_at_token_boundary(text, 20);

    assert!(part1.len() <= 20);
    assert_eq!(format!("{}{}", part1, part2), text);
}
