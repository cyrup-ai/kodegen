use yyaml::YamlLoader;

#[test]
fn test_standalone_block_scalar_debug() {
    println!("=== Debugging Standalone Block Scalar Infinite Loop ===");
    
    let standalone_block_scalar = "|\n  value";
    
    println!("Input: {:?}", standalone_block_scalar);
    println!("Starting parse...");
    
    // This test is expected to hang, so we're just using it for debugging
    match YamlLoader::load_from_str(standalone_block_scalar) {
        Ok(docs) => {
            println!("SUCCESS: Parsed {} documents", docs.len());
            if let Some(doc) = docs.first() {
                println!("Result: {:?}", doc);
            }
        }
        Err(e) => {
            println!("ERROR: {:?}", e);
        }
    }
    
    println!("Parse completed.");
}