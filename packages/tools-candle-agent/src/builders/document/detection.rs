//! Format and media type detection heuristics

use super::types::DocumentBuilderData;
use crate::domain::context::{
    CandleContentFormat as ContentFormat, CandleDocumentChunk as DocumentChunk,
    CandleDocumentMediaType as DocumentMediaType,
};

impl<F1, F2> super::types::DocumentBuilderImpl<F1, F2>
where
    F1: Fn(String) + Send + Sync + 'static + Clone,
    F2: Fn(DocumentChunk) -> DocumentChunk + Send + Sync + 'static + Clone,
{
    #[inline]
    pub(super) fn detect_format(content: &str, data: &DocumentBuilderData) -> ContentFormat {
        match data {
            DocumentBuilderData::File(path) => {
                match path.extension().and_then(|ext| ext.to_str()) {
                    Some("md") | Some("markdown") => ContentFormat::Markdown,
                    Some("html") | Some("htm") => ContentFormat::Html,
                    Some("json") => ContentFormat::Json,
                    Some("xml") => ContentFormat::Xml,
                    Some("yaml") | Some("yml") => ContentFormat::Yaml,
                    Some("csv") => ContentFormat::Csv,
                    _ => ContentFormat::Text,
                }
            }
            DocumentBuilderData::Github { path, .. } => {
                match std::path::Path::new(path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                {
                    Some("md") | Some("markdown") => ContentFormat::Markdown,
                    Some("html") | Some("htm") => ContentFormat::Html,
                    Some("json") => ContentFormat::Json,
                    Some("xml") => ContentFormat::Xml,
                    Some("yaml") | Some("yml") => ContentFormat::Yaml,
                    Some("csv") => ContentFormat::Csv,
                    _ => {
                        // Content-based detection
                        if content.trim_start().starts_with('{')
                            || content.trim_start().starts_with('[')
                        {
                            ContentFormat::Json
                        } else if content.trim_start().starts_with('<') {
                            ContentFormat::Html
                        } else {
                            ContentFormat::Text
                        }
                    }
                }
            }
            DocumentBuilderData::Url(url) => {
                if url.ends_with(".json") {
                    ContentFormat::Json
                } else if url.ends_with(".html") || url.ends_with(".htm") {
                    ContentFormat::Html
                } else if url.ends_with(".md") || url.ends_with(".markdown") {
                    ContentFormat::Markdown
                } else {
                    ContentFormat::Text
                }
            }
            _ => ContentFormat::Text,
        }
    }

    #[inline]
    pub(super) fn detect_media_type(
        format: &ContentFormat,
        data: &DocumentBuilderData,
    ) -> DocumentMediaType {
        match format {
            ContentFormat::Json => DocumentMediaType::Json,
            ContentFormat::Html => DocumentMediaType::Html,
            ContentFormat::Markdown => DocumentMediaType::Markdown,
            ContentFormat::Xml => DocumentMediaType::Xml,
            ContentFormat::Csv => DocumentMediaType::Csv,
            ContentFormat::Yaml => DocumentMediaType::Yaml,
            ContentFormat::Base64 => match data {
                DocumentBuilderData::File(path) => {
                    match path.extension().and_then(|ext| ext.to_str()) {
                        Some("pdf") => DocumentMediaType::PDF,
                        Some("doc") | Some("docx") => DocumentMediaType::Document,
                        Some("jpg") | Some("jpeg") | Some("png") | Some("gif") => {
                            DocumentMediaType::Image
                        }
                        _ => DocumentMediaType::Binary,
                    }
                }
                _ => DocumentMediaType::Binary,
            },
            _ => DocumentMediaType::PlainText,
        }
    }
}
