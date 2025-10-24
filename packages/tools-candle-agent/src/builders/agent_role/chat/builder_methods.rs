//! Simple builder setter methods for CandleAgentBuilder

use super::super::*;

pub(super) fn set_model(
    mut builder: CandleAgentBuilderImpl,
    model: TextToTextModel,
) -> CandleAgentBuilderImpl {
    builder.text_to_text_model = model;
    builder
}

pub(super) fn set_embedding_model(
    mut builder: CandleAgentBuilderImpl,
    model: TextEmbeddingModel,
) -> CandleAgentBuilderImpl {
    builder.text_embedding_model = Some(model);
    builder
}

pub(super) fn set_temperature(
    mut builder: CandleAgentBuilderImpl,
    temp: f64,
) -> CandleAgentBuilderImpl {
    builder.temperature = temp;
    builder
}
pub(super) fn set_max_tokens(
    mut builder: CandleAgentBuilderImpl,
    max: u64,
) -> CandleAgentBuilderImpl {
    builder.max_tokens = max;
    builder
}

pub(super) fn set_memory_read_timeout(
    mut builder: CandleAgentBuilderImpl,
    timeout_ms: u64,
) -> CandleAgentBuilderImpl {
    builder.memory_read_timeout = timeout_ms;
    builder
}

pub(super) fn set_system_prompt(
    mut builder: CandleAgentBuilderImpl,
    prompt: String,
) -> CandleAgentBuilderImpl {
    builder.system_prompt = prompt;
    builder
}

pub(super) fn set_additional_params<P2>(
    mut builder: CandleAgentBuilderImpl,
    params: P2,
) -> CandleAgentBuilderImpl
where
    P2: IntoIterator<Item = (&'static str, &'static str)>,
{
    builder.additional_params.extend(
        params
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string())),
    );
    builder
}
pub(super) fn set_metadata<Meta>(
    mut builder: CandleAgentBuilderImpl,
    metadata: Meta,
) -> CandleAgentBuilderImpl
where
    Meta: IntoIterator<Item = (&'static str, &'static str)>,
{
    builder.metadata.extend(
        metadata
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string())),
    );
    builder
}

pub(super) fn set_context(
    mut builder: CandleAgentBuilderImpl,
    context1: CandleContext<CandleFile>,
    context2: CandleContext<CandleFiles>,
    context3: CandleContext<CandleDirectory>,
    context4: CandleContext<CandleGithub>,
) -> CandleAgentBuilderImpl {
    builder.context_file = Some(context1);
    builder.context_files = Some(context2);
    builder.context_directory = Some(context3);
    builder.context_github = Some(context4);
    builder
}

pub(super) fn set_tools<T>(mut builder: CandleAgentBuilderImpl, tools: T) -> CandleAgentBuilderImpl
where
    T: Into<ZeroOneOrMany<ToolInfo>>,
{
    builder.tools = tools.into();
    builder
}

pub(super) fn add_mcp_server_config_impl(
    builder: CandleAgentBuilderImpl,
    _config: McpServerConfig,
) -> CandleAgentBuilderImpl {
    // MCP servers are handled through tools
    builder
}

pub(super) fn set_stop_sequences(
    mut builder: CandleAgentBuilderImpl,
    sequences: Vec<String>,
) -> CandleAgentBuilderImpl {
    builder.stop_sequences = sequences;
    builder
}

pub(super) fn add_stop_sequence_impl(
    mut builder: CandleAgentBuilderImpl,
    sequence: String,
) -> CandleAgentBuilderImpl {
    builder.stop_sequences.push(sequence);
    builder
}
