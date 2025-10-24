//! API models and request/response types
//! This module contains the data structures used by the API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::memory::primitives::types::MemoryTypeEnum;

/// Request to create a new memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemoryRequest {
    pub content: String,
    pub memory_type: MemoryTypeEnum,
    pub metadata: Option<serde_json::Value>,
    pub user_id: Option<String>,
}

/// Response containing memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResponse {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryTypeEnum,
    pub metadata: Option<serde_json::Value>,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub memory_type: Option<MemoryTypeEnum>,
    pub user_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}
