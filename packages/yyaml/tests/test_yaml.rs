fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();
    let yaml_content = r#"
# Links:
#  - https://platform.openai.com/docs/models
#  - https://platform.openai.com/docs/api-reference/chat
- provider: openai
  models:
    - name: gpt-4.1
      max_input_tokens: 1047576
      max_output_tokens: 32768
      input_price: 2
      output_price: 8
      supports_vision: true
      supports_function_calling: true
    - name: o4-mini
      max_input_tokens: 200000
      input_price: 1.1
      output_price: 4.4
      supports_vision: true
      supports_function_calling: true
      system_prompt_prefix: Formatting re-enabled
      patch:
        body:
          max_tokens: null
          temperature: null
          top_p: null
"#;

    match yyaml::YamlLoader::load_from_str(yaml_content) {
        Ok(docs) => {
            println!("✅ YAML parsing succeeded! Parsed {} documents", docs.len());
            if let Some(doc) = docs.first() {
                println!("First document: {doc:?}");
            }
        }
        Err(e) => {
            println!("❌ YAML parsing failed: {e:?}");
            std::process::exit(1);
        }
    }
}
