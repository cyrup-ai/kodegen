use kodegen_candle_agent::domain::voice::transcription::{
    TranscriptionRequest, TranscriptionResponse,
};

#[test]
fn test_transcription_request_creation() {
    let data = vec![1, 2, 3];
    let request = TranscriptionRequest::new(data.clone(), "test.mp3", "en");

    assert_eq!(request.data, data);
    assert_eq!(request.filename, "test.mp3");
    assert_eq!(request.language, "en");
    assert!(request.prompt.is_none());
    assert!(request.temperature.is_none());
    assert!(request.additional_params.is_none());
}

#[test]
fn test_transcription_request_builder() {
    let data = vec![1, 2, 3];
    let params = serde_json::json!({ "model": "whisper-1" });

    let request = TranscriptionRequest::new(data, "test.mp3", "en")
        .with_prompt("Transcribe this audio")
        .with_temperature(0.7)
        .with_additional_params(params.clone());

    assert_eq!(request.prompt, Some("Transcribe this audio".to_string()));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.additional_params, Some(params));
}

#[test]
fn test_transcription_response() {
    let response = TranscriptionResponse::new("test".to_string(), ());

    assert_eq!(response.text(), "test");
    assert_eq!(response.into_inner(), ());
}

#[test]
fn test_transcription_response_map() {
    let response = TranscriptionResponse::new("test".to_string(), 42);
    let mapped = response.map(|x| x.to_string());

    assert_eq!(mapped.text, "test");
    assert_eq!(mapped.response, "42");
}
