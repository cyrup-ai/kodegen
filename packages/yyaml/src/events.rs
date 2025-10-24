use crate::error::Marker;

/// Event signals from the parser
#[derive(Clone, PartialEq, Debug, Eq)]
pub enum Event {
    Nothing,
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    YamlDirective(u32, u32),
    TagDirective(String, String),
    Alias(usize),
    Scalar(String, TScalarStyle, usize, Option<TokenType>),
    SequenceStart(usize),
    SequenceEnd,
    MappingStart(usize),
    MappingEnd,
}

/// Minimally track scalar style
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum TScalarStyle {
    Any,
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

#[derive(Clone, PartialEq, Debug, Eq)]
pub enum TokenType {
    Tag(String, String),
    Anchor(String),
    Alias(String),
    Scalar(TScalarStyle, String),
    DocumentStart,
    DocumentEnd,
    FlowSequenceStart,
    FlowSequenceEnd,
    FlowMappingStart,
    FlowMappingEnd,
    BlockSequenceStart,
    BlockMappingStart,
    BlockEnd,
    Key,
    Value,
    FlowEntry,
    BlockEntry,
    StreamStart(TEncoding),
    StreamEnd,
    VersionDirective(u32, u32),
    TagDirective(String, String),
    Reserved(String),
    NoToken,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum TEncoding {
    Utf8,
}

/// A trait for receiving parser events. We provide our own YamlReceiver that
/// constructs `Yaml` documents from events.
pub trait MarkedEventReceiver {
    fn on_event(&mut self, ev: Event, mark: Marker);
}

/// Simplified trait: ignoring marker
pub trait EventReceiver {
    fn on_event(&mut self, ev: Event);
}

impl<R: EventReceiver> MarkedEventReceiver for R {
    fn on_event(&mut self, ev: Event, _mark: Marker) {
        self.on_event(ev);
    }
}
