use yyaml::YamlLoader;

fn main() {
    let s = "key: value";
    println!("Trying to parse: {s:?}");

    let result = YamlLoader::load_from_str(s);
    match result {
        Ok(docs) => {
            println!("Success! Got {} documents", docs.len());
            for (i, doc) in docs.iter().enumerate() {
                println!("Document {i}: {doc:?}");
            }
        }
        Err(e) => {
            println!("Error: {e}");
            println!("Error debug: {e:?}");
        }
    }
}
