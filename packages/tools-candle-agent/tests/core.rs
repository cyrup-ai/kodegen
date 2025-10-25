// Integration tests for core operations

mod core {
    mod generation {
        mod test_types;
        mod test_stats;
        mod test_tokens;
        mod test_config;
    }
    mod test_model_config;
    mod test_simd_adapters;
    mod tokenizer {
        mod test_core;
    }
}
