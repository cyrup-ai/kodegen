//! Port assignments and routing table for category HTTP servers.

use std::collections::HashMap;
use once_cell::sync::Lazy;

use kodegen_mcp_schema::ToolMetadata;

/// Re-export canonical port assignments from kodegen-config (single source of truth)
pub use kodegen_config::CATEGORY_PORTS;

/// Global routing table: tool_name -> (category, port)
///
/// Initialized lazily on first access. Built once and reused across all server instances.
/// Contains mappings for all tools to their respective category servers and ports.
static ROUTING_TABLE: Lazy<HashMap<&'static str, (&'static str, u16)>> = Lazy::new(|| {
    let mut table = HashMap::new();
    let port_map: HashMap<&str, u16> = CATEGORY_PORTS
        .iter()
        .map(|(cat, port)| (cat.name, *port))
        .collect();

    for tool in inventory::iter::<ToolMetadata>() {
        if let Some(&port) = port_map.get(tool.category.name) {
            table.insert(tool.name, (tool.category.name, port));
        }
    }

    table
});

/// Get the global routing table.
/// 
/// Returns a reference to the lazily-initialized static routing table.
/// First call initializes the table, subsequent calls return cached reference.
pub fn get_routing_table() -> &'static HashMap<&'static str, (&'static str, u16)> {
    &ROUTING_TABLE
}
