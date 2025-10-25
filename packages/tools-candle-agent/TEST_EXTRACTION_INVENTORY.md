# Test Extraction Inventory

**Package**: `tools-candle-agent`
**Total files with tests**: 40
**Status**: In Progress
**Last Updated**: 2025-10-25

## Extraction Rules

1. Tests extracted from `src/` to `tests/` with mirrored directory structure
2. Test file naming: `test_{original_filename}.rs`
3. Example: `src/memory/core/mod.rs` → `tests/memory/core/test_mod.rs`
4. All tests run with `cargo nextest run`

## Files To Extract (40 total)

### lib.rs (1 file)
- [ ] `lib.rs` → `tests/test_lib.rs`

### builders/ (2 files)
- [ ] `builders/embedding.rs` → `tests/builders/test_embedding.rs`
- [ ] `builders/vision/mod.rs` → `tests/builders/vision/test_mod.rs`

### capability/ (3 files)
- [ ] `capability/registry/mod.rs` → `tests/capability/registry/test_mod.rs`
- [ ] `capability/registry/tests.rs` → `tests/capability/registry/test_tests.rs`
- [ ] `capability/text_embedding/stella/instruction.rs` → `tests/capability/text_embedding/stella/test_instruction.rs`

### cli/ (1 file)
- [ ] `cli/handler.rs` → `tests/cli/test_handler.rs`

### context/ (2 files)
- [ ] `context/extraction/error.rs` → `tests/context/extraction/test_error.rs`
- [ ] `context/extraction/mod.rs` → `tests/context/extraction/test_mod.rs`

### core/ (6 files)
- [ ] `core/generation/config.rs` → `tests/core/generation/test_config.rs`
- [ ] `core/generation/stats.rs` → `tests/core/generation/test_stats.rs`
- [ ] `core/generation/tokens.rs` → `tests/core/generation/test_tokens.rs`
- [ ] `core/generation/types.rs` → `tests/core/generation/test_types.rs`
- [ ] `core/model_config.rs` → `tests/core/test_model_config.rs`
- [ ] `core/simd_adapters.rs` → `tests/core/test_simd_adapters.rs`
- [ ] `core/tokenizer/core.rs` → `tests/core/tokenizer/test_core.rs`

### domain/chat/ (4 files)
- [ ] `domain/chat/loop.rs` → `tests/domain/chat/test_loop.rs`
- [ ] `domain/chat/message/message_processing.rs` → `tests/domain/chat/message/test_message_processing.rs`
- [ ] `domain/chat/message/mod.rs` → `tests/domain/chat/message/test_mod.rs`
- [ ] `domain/chat/orchestration.rs` → `tests/domain/chat/test_orchestration.rs`
- [ ] `domain/chat/templates/parser/mod.rs` → `tests/domain/chat/templates/parser/test_mod.rs`

### domain/completion/ (1 file)
- [ ] `domain/completion/prompt_formatter.rs` → `tests/domain/completion/test_prompt_formatter.rs`

### domain/context/ (1 file)
- [ ] `domain/context/extraction/mod.rs` → `tests/domain/context/extraction/test_mod.rs`

### domain/model/ (1 file)
- [ ] `domain/model/error.rs` → `tests/domain/model/test_error.rs`

### domain/util/ (1 file)
- [ ] `domain/util/json_util.rs` → `tests/domain/util/test_json_util.rs`

### memory/core/ (2 files)
- [ ] `memory/core/mod.rs` → `tests/memory/core/test_mod.rs`
- [ ] `memory/core/tests/schema.rs` → `tests/memory/core/test_schema.rs` (already in tests subdir)

### memory/migration/ (1 file)
- [ ] `memory/migration/converter.rs` → `tests/memory/migration/test_converter.rs`

### memory/monitoring/ (3 files)
- [ ] `memory/monitoring/metrics_test.rs` → `tests/memory/monitoring/test_metrics_test.rs`
- [ ] `memory/monitoring/metrics.rs` → `tests/memory/monitoring/test_metrics.rs`
- [ ] `memory/monitoring/mod.rs` → `tests/memory/monitoring/test_mod.rs`
- [ ] `memory/monitoring/tests/metrics_tests.rs` → `tests/memory/monitoring/test_metrics_tests.rs` (already in tests subdir)

### memory/schema/ (1 file)
- [ ] `memory/schema/relationship_schema.rs` → `tests/memory/schema/test_relationship_schema.rs`

### memory/transaction/ (2 files)
- [ ] `memory/transaction/mod.rs` → `tests/memory/transaction/test_mod.rs`
- [ ] `memory/transaction/tests/transaction_manager_tests.rs` → `tests/memory/transaction/test_transaction_manager_tests.rs` (already in tests subdir)

### memory/vector/ (2 files)
- [ ] `memory/vector/vector_index.rs` → `tests/memory/vector/test_vector_index.rs`
- [ ] `memory/vector/vector_repository.rs` → `tests/memory/vector/test_vector_repository.rs`

### util/ (2 files)
- [ ] `util/input_resolver.rs` → `tests/util/test_input_resolver.rs`
- [ ] `util/json_util.rs` → `tests/util/test_json_util.rs`

### workflow/ (1 file)
- [ ] `workflow/parallel.rs` → `tests/workflow/test_parallel.rs`

## Progress Tracking

- **Completed**: 0/40
- **In Progress**: 0/40
- **Remaining**: 40/40

## Notes

- Some files already have tests subdirectories within src (e.g., `memory/monitoring/tests/`)
- These will be moved to top-level `tests/` directory
- After extraction, test modules should be removed from src files
- Each extraction should be verified with `cargo nextest run` before proceeding
