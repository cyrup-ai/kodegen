use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Fast hash function for content-based embedding generation
#[inline]
#[must_use]
pub fn content_hash(content: &str) -> i64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish().cast_signed() // Cast to i64 for SurrealDB compatibility
}

/// Zero-allocation binary format for memory records
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub input_hash: i64,
    pub output_hash: i64,
    pub timestamp: u64,
    pub input_length: u32,
    pub output_length: u32,
}

impl MemoryRecord {
    /// Create new memory record with zero allocation
    #[inline]
    #[must_use]
    pub fn new(input: &str, output: &str, timestamp: u64) -> Self {
        Self {
            input_hash: content_hash(input),
            output_hash: content_hash(output),
            timestamp,
            input_length: u32::try_from(input.len()).unwrap_or(u32::MAX),
            output_length: u32::try_from(output.len()).unwrap_or(u32::MAX),
        }
    }

    /// Serialize to binary format with zero allocation
    #[inline]
    pub fn serialize_to_buffer(&self, buffer: &mut SerializationBuffer) {
        buffer.clear();
        buffer.write_i64(self.input_hash);
        buffer.write_i64(self.output_hash);
        buffer.write_u64(self.timestamp);
        buffer.write_u32(self.input_length);
        buffer.write_u32(self.output_length);
    }

    /// Deserialize from binary format with zero allocation
    #[inline]
    #[must_use]
    pub fn deserialize_from_buffer(buffer: &SerializationBuffer) -> Option<Self> {
        if buffer.data.len() < 32 {
            // 8+8+8+4+4 = 32 bytes
            return None;
        }

        let mut pos = 0;
        let input_hash = i64::from_le_bytes(buffer.data[pos..pos + 8].try_into().ok()?);
        pos += 8;
        let output_hash = i64::from_le_bytes(buffer.data[pos..pos + 8].try_into().ok()?);
        pos += 8;
        let timestamp = u64::from_le_bytes(buffer.data[pos..pos + 8].try_into().ok()?);
        pos += 8;
        let input_length = u32::from_le_bytes(buffer.data[pos..pos + 4].try_into().ok()?);
        pos += 4;
        let output_length = u32::from_le_bytes(buffer.data[pos..pos + 4].try_into().ok()?);

        Some(Self {
            input_hash,
            output_hash,
            timestamp,
            input_length,
            output_length,
        })
    }

    /// Format as string for storage (minimal allocation)
    #[inline]
    #[must_use]
    pub fn to_content_string(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.input_hash,
            self.output_hash,
            self.timestamp,
            self.input_length,
            self.output_length
        )
    }
}

/// Zero-allocation serialization buffer with pre-allocated capacity
#[derive(Debug)]
pub struct SerializationBuffer {
    data: Vec<u8>,
    capacity: usize,
}

impl SerializationBuffer {
    /// Create new buffer with pre-allocated capacity
    #[inline]
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Clear buffer for reuse (zero allocation)
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Write u64 in little-endian format
    #[inline]
    pub fn write_u64(&mut self, value: u64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    /// Write u32 in little-endian format
    #[inline]
    pub fn write_u32(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    /// Write i64 in little-endian format
    #[inline]
    pub fn write_i64(&mut self, value: i64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    /// Get buffer data as slice
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Get buffer length
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if buffer is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Reserve additional capacity if needed
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        if self.data.len() + additional > self.capacity {
            self.data.reserve(additional);
            self.capacity = self.data.capacity();
        }
    }
}

impl Default for SerializationBuffer {
    #[inline]
    fn default() -> Self {
        Self::new(256) // Default 256 bytes capacity
    }
}

// Thread-local serialization buffer pool for zero-allocation operations
thread_local! {
    static SERIALIZATION_BUFFER: std::cell::RefCell<SerializationBuffer> =
        std::cell::RefCell::new(SerializationBuffer::new(1024));
}

/// Get thread-local serialization buffer for zero-allocation operations
#[inline]
pub fn with_serialization_buffer<F, R>(f: F) -> R
where
    F: FnOnce(&mut SerializationBuffer) -> R,
{
    SERIALIZATION_BUFFER.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        f(&mut buffer)
    })
}
