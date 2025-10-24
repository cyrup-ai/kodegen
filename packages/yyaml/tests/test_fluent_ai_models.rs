use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProviderInfo {
    provider: String,
    models: Vec<ModelConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ModelConfig {
    name: String,
    #[serde(default)]
    max_input_tokens: Option<u64>,
    #[serde(default)]
    max_output_tokens: Option<u64>,
    #[serde(default)]
    input_price: Option<f64>,
    #[serde(default)]
    output_price: Option<f64>,
    #[serde(default)]
    supports_vision: Option<bool>,
    #[serde(default)]
    supports_function_calling: Option<bool>,
    #[serde(default)]
    require_max_tokens: Option<bool>,
}

fn main() {
    let yaml_content = r#"- provider: OpenAI
  models:
    - name: gpt-3.5-turbo
      max_input_tokens: 16385
      max_output_tokens: 4096
      supports_function_calling: true
      supports_vision: false
      require_max_tokens: false"#;

    println!("Testing YAML parsing with yyaml::from_str...");

    // Test with the custom yyaml crate
    match yyaml::from_str::<Vec<ProviderInfo>>(yaml_content) {
        Ok(providers) => {
            println!("✅ SUCCESS: Parsed {} providers", providers.len());
            for provider in providers {
                println!(
                    "  Provider: {}, Models: {}",
                    provider.provider,
                    provider.models.len()
                );
            }
        }
        Err(e) => {
            println!("❌ ERROR: {e}");
        }
    }
}
