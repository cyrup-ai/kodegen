use kodegen_candle_agent::domain::voice::{Audio, TranscriptionResponse};

#[test]
fn test_audio_creation() {
    let audio = Audio::new("test".to_string());
    assert_eq!(audio.data, "test");
    assert!(audio.format.is_none());
    assert!(audio.media_type.is_none());
}

#[test]
fn test_transcription_response() {
    let response = TranscriptionResponse::new("test".to_string(), ());
    assert_eq!(response.text(), "test");
}
