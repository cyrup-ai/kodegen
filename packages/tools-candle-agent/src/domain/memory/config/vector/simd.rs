use serde::{Deserialize, Serialize};

/// SIMD instruction sets for vector operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SimdInstructionSet {
    /// No SIMD optimization
    None = 0,
    /// SSE (128-bit)
    SSE = 1,
    /// AVX (256-bit)
    AVX = 2,
    /// AVX2 (256-bit with integer support)
    AVX2 = 3,
    /// AVX-512 (512-bit)
    AVX512 = 4,
    /// ARM NEON (128-bit)
    NEON = 5,
}

impl SimdInstructionSet {
    /// Get vector width in bytes
    #[inline]
    #[must_use]
    pub const fn vector_width_bytes(&self) -> usize {
        match self {
            Self::None => 4,              // Single f32
            Self::SSE | Self::NEON => 16, // 128-bit
            Self::AVX | Self::AVX2 => 32, // 256-bit
            Self::AVX512 => 64,           // 512-bit
        }
    }

    /// Get number of f32 elements per vector
    #[inline]
    #[must_use]
    pub const fn f32_elements_per_vector(&self) -> usize {
        self.vector_width_bytes() / 4
    }

    /// Check if instruction set is available on current CPU
    #[must_use]
    pub fn is_available(&self) -> bool {
        match self {
            Self::None => true,
            #[cfg(target_arch = "x86_64")]
            Self::SSE => is_x86_feature_detected!("sse"),
            #[cfg(target_arch = "x86_64")]
            Self::AVX => is_x86_feature_detected!("avx"),
            #[cfg(target_arch = "x86_64")]
            Self::AVX2 => is_x86_feature_detected!("avx2"),
            #[cfg(target_arch = "x86_64")]
            Self::AVX512 => is_x86_feature_detected!("avx512f"),
            #[cfg(target_arch = "aarch64")]
            Self::NEON => std::arch::is_aarch64_feature_detected!("neon"),
            #[cfg(not(target_arch = "x86_64"))]
            Self::SSE | Self::AVX | Self::AVX2 | Self::AVX512 => false,
            #[cfg(not(target_arch = "aarch64"))]
            Self::NEON => false,
        }
    }

    /// Detect best available SIMD instruction set
    #[must_use]
    pub fn detect_best_available() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            if Self::AVX512.is_available() {
                Self::AVX512
            } else if Self::AVX2.is_available() {
                Self::AVX2
            } else if Self::AVX.is_available() {
                Self::AVX
            } else if Self::SSE.is_available() {
                Self::SSE
            } else {
                Self::None
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if Self::NEON.is_available() {
                Self::NEON
            } else {
                Self::None
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        Self::None
    }
}

/// SIMD configuration for vector operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimdConfig {
    /// Target SIMD instruction set
    pub instruction_set: SimdInstructionSet,
    /// Enable auto-detection of best SIMD support
    pub auto_detect: bool,
    /// Force specific alignment for vectors
    pub force_alignment: Option<usize>,
    /// Enable SIMD for distance calculations
    pub enable_distance_simd: bool,
    /// Enable SIMD for vector normalization
    pub enable_normalization_simd: bool,
    /// Minimum vector dimension to use SIMD
    pub simd_threshold: usize,
}

impl SimdConfig {
    /// Create optimized SIMD configuration
    #[inline]
    #[must_use]
    pub fn optimized() -> Self {
        let instruction_set = {
            // Removed unexpected cfg condition "simd-auto-detect" - feature does not exist
            SimdInstructionSet::None
        };

        Self {
            instruction_set,
            auto_detect: true,
            force_alignment: Some(32), // 256-bit alignment for AVX2
            enable_distance_simd: true,
            enable_normalization_simd: true,
            simd_threshold: 16, // Use SIMD for vectors >= 16 dimensions
        }
    }

    /// Create disabled SIMD configuration
    #[inline]
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            instruction_set: SimdInstructionSet::None,
            auto_detect: false,
            force_alignment: None,
            enable_distance_simd: false,
            enable_normalization_simd: false,
            simd_threshold: usize::MAX,
        }
    }

    /// Check if SIMD should be used for given dimension
    #[inline]
    #[must_use]
    pub fn should_use_simd(&self, dimension: usize) -> bool {
        self.instruction_set != SimdInstructionSet::None
            && dimension >= self.simd_threshold
            && (self.enable_distance_simd || self.enable_normalization_simd)
    }

    /// Get optimal alignment for vectors
    #[inline]
    #[must_use]
    pub fn optimal_alignment(&self) -> usize {
        self.force_alignment
            .unwrap_or_else(|| self.instruction_set.vector_width_bytes())
            .max(4) // Minimum f32 alignment
    }
}

impl Default for SimdConfig {
    #[inline]
    fn default() -> Self {
        Self::optimized()
    }
}
