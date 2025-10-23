//! Test to verify when .`collect()` is necessary vs when direct iteration is safe
//! for DOM manipulation in inline CSS utilities.
//!
//! This test documents findings from task inline-css-008-premature-collect.md
//!
//! # Summary
//!
//! `.collect()` is necessary when calling `node.detach()` during iteration,
//! but can be avoided when only modifying attributes.
//!
//! ## Functions that NEED .`collect()`:
//! - `replace_css_links_with_styles()` - calls `node.detach()`
//! - `replace_img_tags_with_svg()` - calls `node.detach()`
//! - `apply_all_replacements()` - calls `node.detach()` for CSS and SVG
//!
//! ## Functions that DON'T need .`collect()` (optimization applied):
//! - `replace_image_sources()` - only modifies attributes

#[cfg(test)]
mod inline_css_iterator_tests {
    use kuchiki::traits::*;

    #[test]
    fn test_collect_required_for_node_detachment() {
        // Demonstrates that .collect() IS necessary when detaching nodes during iteration
        let html = r#"
            <html>
                <head>
                    <link rel="stylesheet" href="style1.css">
                    <link rel="stylesheet" href="style2.css">
                    <link rel="stylesheet" href="style3.css">
                </head>
            </html>
        "#;

        let document = kuchiki::parse_html().one(html);

        // WITH .collect() - iterates all nodes correctly
        let matches: Vec<_> = document
            .select("link[rel=\"stylesheet\"]")
            .unwrap()
            .collect();
        let mut count = 0;
        for node_ref in matches {
            let node = node_ref.as_node();
            node.detach();
            count += 1;
        }

        assert_eq!(count, 3, "With collect(), should iterate all 3 nodes");

        // Verify all nodes were removed
        let remaining = document.select("link[rel=\"stylesheet\"]").unwrap().count();
        assert_eq!(remaining, 0, "All link elements should be removed");
    }

    #[test]
    fn test_direct_iteration_safe_for_attribute_modification() {
        // Demonstrates that direct iteration (no .collect()) is SAFE for attribute modification
        let html = r#"
            <html>
                <body>
                    <img src="image1.jpg">
                    <img src="image2.jpg">
                    <img src="image3.jpg">
                </body>
            </html>
        "#;

        let document = kuchiki::parse_html().one(html);

        // Direct iteration WITHOUT .collect() - safe for attribute changes
        let mut modified_count = 0;
        for node_ref in document.select("img[src]").unwrap() {
            let mut attrs = node_ref.attributes.borrow_mut();
            let old_src = attrs.get("src").unwrap().to_string();
            attrs.insert("src", format!("data:image/png;base64,{old_src}"));
            modified_count += 1;
        }

        assert_eq!(
            modified_count, 3,
            "Direct iteration works for attribute modification"
        );

        // Verify modifications were applied
        for node_ref in document.select("img[src]").unwrap() {
            let attrs = node_ref.attributes.borrow();
            let src = attrs.get("src").unwrap();
            assert!(
                src.starts_with("data:image/png;base64,"),
                "Src should be data URL"
            );
        }
    }

    #[test]
    fn test_collect_required_for_node_replacement() {
        // Demonstrates that .collect() IS necessary when replacing nodes during iteration
        let html = r#"
            <html>
                <body>
                    <img src="icon1.svg">
                    <img src="icon2.svg">
                </body>
            </html>
        "#;

        let document = kuchiki::parse_html().one(html);

        // WITH .collect() - iterates all nodes correctly
        let matches: Vec<_> = document.select("img[src]").unwrap().collect();
        let mut replaced_count = 0;
        for node_ref in matches {
            let node = node_ref.as_node();

            // Create replacement SVG
            let svg_html = "<svg><circle r='10'/></svg>";
            let svg_fragment = kuchiki::parse_html().one(svg_html);

            // Insert SVG before img
            for child in svg_fragment.children() {
                node.insert_before(child);
            }

            // Remove img
            node.detach();
            replaced_count += 1;
        }

        assert_eq!(
            replaced_count, 2,
            "With collect(), should replace both img elements"
        );

        // Verify replacements
        let img_count = document.select("img").unwrap().count();
        let svg_count = document.select("svg").unwrap().count();
        assert_eq!(img_count, 0, "No img elements should remain");
        assert_eq!(svg_count, 2, "Should have 2 SVG elements");
    }
}
