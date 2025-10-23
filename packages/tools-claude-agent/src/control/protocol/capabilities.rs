//! Client and server capability definitions
//!
//! This module defines capability flags used in protocol negotiation.

use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    /// Client capabilities for negotiation
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ClientCapabilities: u8 {
        /// Supports bidirectional communication
        const BIDIRECTIONAL = 0b0001;
        /// Supports hooks
        const HOOKS = 0b0010;
        /// Supports permissions
        const PERMISSIONS = 0b0100;
        /// Supports interrupts
        const INTERRUPTS = 0b1000;
    }
}

impl ClientCapabilities {
    /// Create capabilities with all features enabled
    #[must_use]
    pub const fn all_features() -> Self {
        Self::all()
    }
}

// Custom serialization to maintain JSON compatibility with boolean fields
impl Serialize for ClientCapabilities {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ClientCapabilities", 4)?;
        state.serialize_field("bidirectional", &self.contains(Self::BIDIRECTIONAL))?;
        state.serialize_field("hooks", &self.contains(Self::HOOKS))?;
        state.serialize_field("permissions", &self.contains(Self::PERMISSIONS))?;
        state.serialize_field("interrupts", &self.contains(Self::INTERRUPTS))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ClientCapabilities {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[allow(clippy::struct_excessive_bools)] // APPROVED BY DAVID MAPLE on 2025-10-13: Required for JSON deserialization compatibility
        struct Helper {
            bidirectional: bool,
            hooks: bool,
            permissions: bool,
            interrupts: bool,
        }

        let h = Helper::deserialize(deserializer)?;
        let mut caps = Self::empty();
        if h.bidirectional {
            caps |= Self::BIDIRECTIONAL;
        }
        if h.hooks {
            caps |= Self::HOOKS;
        }
        if h.permissions {
            caps |= Self::PERMISSIONS;
        }
        if h.interrupts {
            caps |= Self::INTERRUPTS;
        }
        Ok(caps)
    }
}

bitflags! {
    /// Server capabilities advertised by CLI
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ServerCapabilities: u8 {
        /// Supports streaming responses
        const STREAMING = 0b001;
        /// Supports tool use
        const TOOLS = 0b010;
        /// Supports MCP servers
        const MCP = 0b100;
    }
}

impl ServerCapabilities {
    /// Create capabilities with all features enabled
    #[must_use]
    pub const fn all_features() -> Self {
        Self::all()
    }
}

// Custom serialization to maintain JSON compatibility with boolean fields
impl Serialize for ServerCapabilities {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ServerCapabilities", 3)?;
        state.serialize_field("streaming", &self.contains(Self::STREAMING))?;
        state.serialize_field("tools", &self.contains(Self::TOOLS))?;
        state.serialize_field("mcp", &self.contains(Self::MCP))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ServerCapabilities {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            streaming: bool,
            tools: bool,
            mcp: bool,
        }

        let h = Helper::deserialize(deserializer)?;
        let mut caps = Self::empty();
        if h.streaming {
            caps |= Self::STREAMING;
        }
        if h.tools {
            caps |= Self::TOOLS;
        }
        if h.mcp {
            caps |= Self::MCP;
        }
        Ok(caps)
    }
}
