use kodegen_candle_agent::domain::voice::audio::{Audio, AudioMediaType, ContentFormat};

#[test]
fn test_audio_creation() {
    let audio = Audio::new("test");
    assert_eq!(audio.data, "test");
    assert!(audio.format.is_none());
    assert!(audio.media_type.is_none());
}

#[test]
fn test_audio_builder_pattern() {
    let audio = Audio::new("test")
        .with_format(ContentFormat::Base64)
        .with_media_type(AudioMediaType::MP3);

    assert_eq!(audio.data, "test");
    assert_eq!(audio.format, Some(ContentFormat::Base64));
    assert_eq!(audio.media_type, Some(AudioMediaType::MP3));
}

#[test]
fn test_audio_format_checks() {
    let base64_audio = Audio::new("dGVzdA==").with_format(ContentFormat::Base64);
    let raw_audio = Audio::new("raw").with_format(ContentFormat::Raw);
    let url_audio = Audio::new("https://example.com/audio.mp3").with_format(ContentFormat::Url);

    assert!(base64_audio.is_base64());
    assert!(raw_audio.is_raw());
    assert!(url_audio.is_url());
}
