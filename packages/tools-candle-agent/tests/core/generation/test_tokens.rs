// Tests extracted from src/core/generation/tokens.rs

use kodegen_candle_agent::core::generation::tokens::{SpecialTokens, TokenProb, TokenHistory};

#[test]
fn test_special_tokens() {
    let tokens = SpecialTokens::with_eos(2);
    assert!(tokens.is_eos(2));
    assert!(!tokens.is_eos(1));
    assert!(!tokens.is_bos(2));
}

#[test]
fn test_token_prob_ordering() {
    let token1 = TokenProb::new(1, 0.8);
    let token2 = TokenProb::new(2, 0.6);

    // Higher probability should sort first (reverse order)
    assert!(token1 < token2);

    let mut tokens = [token2, token1];
    tokens.sort();
    assert_eq!(tokens[0].prob, 0.8);
}

#[test]
fn test_token_history() {
    let mut history = TokenHistory::new(3);

    history.push(1);
    history.push(2);
    history.push(3);
    assert_eq!(history.len(), 3);

    history.push(4); // Should evict token 1
    assert_eq!(history.len(), 3);
    assert_eq!(history.as_slice(), vec![2, 3, 4]);
}
