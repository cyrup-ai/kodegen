//! Port assignments and routing table for category HTTP servers.

use std::collections::HashMap;
use once_cell::sync::Lazy;

use super::category_metadata::all_tool_metadata;

/// Port assignments for category HTTP servers (matches daemon config.rs allocation).
pub const CATEGORY_PORTS: &[(&str, u16)] = &[
    ("browser", 30438),
    ("candle_agent", 30452),
    ("citescrape", 30439),
    ("claude_agent", 30440),
    ("config", 30441),
    ("database", 30442),
    ("filesystem", 30443),
    ("git", 30444),
    ("github", 30445),
    ("introspection", 30446),
    ("process", 30447),
    ("prompt", 30448),
    ("reasoner", 30449),
    ("sequential_thinking", 30450),
    ("terminal", 30451),
];

/// Global routing table: tool_name -> (category, port)
/// 
/// Initialized lazily on first access. Built once and reused across all server instances.
/// Contains mappings for all ~109 tools to their respective category servers and ports.
static ROUTING_TABLE: Lazy<HashMap<&'static str, (&'static str, u16)>> = Lazy::new(|| {
    let mut table = HashMap::new();
    let port_map: HashMap<&str, u16> = CATEGORY_PORTS.iter().copied().collect();
    
    for tool in all_tool_metadata() {
        if let Some(&port) = port_map.get(tool.category) {
            table.insert(tool.name, (tool.category, port));
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
