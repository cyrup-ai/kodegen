use yyaml::scanner::Scanner;

#[test]
fn test_bom_breaks_whitespace_skipping() {
    println!("=== Testing BOM impact on whitespace skipping ===");

    // Test 1: Without BOM - should work correctly
    let input_no_bom = "- 0\n";
    let mut scanner = Scanner::new(input_no_bom.chars());

    // Skip stream start token
    let stream_start = scanner.fetch_token();
    println!("No BOM - Stream start: {:?}", stream_start);

    // Get first content token
    let first_content = scanner.peek_token().unwrap();
    println!("No BOM - First content token: {:?}", first_content);

    // Test 2: With BOM - demonstrates the bug
    let input_with_bom = "\u{feff}- 0\n";
    let mut scanner2 = Scanner::new(input_with_bom.chars());

    // Skip stream start token
    let stream_start2 = scanner2.fetch_token();
    println!("With BOM - Stream start: {:?}", stream_start2);

    // Get first content token - this will show the bug
    let first_content2 = scanner2.peek_token().unwrap();
    println!("With BOM - First content token: {:?}", first_content2);

    println!("=== BUG DEMONSTRATION ===");
    println!("Without BOM: {:?}", first_content);
    println!("With BOM:    {:?}", first_content2);

    // The bug: BOM causes different token types because whitespace skipping fails
    if format!("{:?}", first_content) != format!("{:?}", first_content2) {
        println!("❌ BUG CONFIRMED: BOM changes token parsing behavior");
        println!("   This proves BOM is not being handled in whitespace skipping");
    } else {
        println!("✅ No difference detected");
    }
}

#[test]
fn test_scanner_character_dispatch_with_bom() {
    println!("=== Testing character dispatch with BOM ===");

    let input = "\u{feff}- 0\n";
    let mut scanner = Scanner::new(input.chars());

    // Skip stream start
    let stream_start = scanner.fetch_token();
    println!("Stream start token: {:?}", stream_start);

    // The next token should be sequence-related, not scalar
    let next_token = scanner.peek_token().unwrap();
    println!("Character dispatch result: {:?}", next_token);

    // Check if this looks like a scalar token (indicating BOM broke dispatch)
    let token_debug = format!("{:?}", next_token);
    if token_debug.contains("Scalar") {
        println!("❌ BOM BREAKS CHARACTER DISPATCH");
        println!("   Scanner is seeing BOM character instead of '-' for sequence");
        println!("   This proves whitespace skipping doesn't handle BOM");
    } else {
        println!("✅ Character dispatch working correctly");
    }
}

#[test]
fn test_complete_parsing_pipeline_with_bom() {
    println!("=== Testing complete parsing pipeline ===");

    use yyaml::parser::YamlLoader;

    // Test the complete pipeline: Scanner → Parser → YamlReceiver → Yaml structure
    let input_with_bom = "\u{feff}- 0\n";
    let input_no_bom = "- 0\n";

    println!("Testing full parsing pipeline...");

    // Parse with BOM
    match YamlLoader::load_from_str(input_with_bom) {
        Ok(docs_bom) => {
            println!("With BOM - Parsed docs: {:?}", docs_bom);
            if let Some(first_doc) = docs_bom.first() {
                println!("With BOM - First doc: {:?}", first_doc);
            }
        }
        Err(e) => println!("With BOM - Parse error: {:?}", e),
    }

    // Parse without BOM
    match YamlLoader::load_from_str(input_no_bom) {
        Ok(docs_no_bom) => {
            println!("No BOM - Parsed docs: {:?}", docs_no_bom);
            if let Some(first_doc) = docs_no_bom.first() {
                println!("No BOM - First doc: {:?}", first_doc);
            }
        }
        Err(e) => println!("No BOM - Parse error: {:?}", e),
    }

    // This will show us exactly where the pipeline breaks
}
