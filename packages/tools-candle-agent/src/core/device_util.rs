//! GPU device detection utilities for Candle framework
//!
//! Provides intelligent device selection with priority:
//! 1. CUDA (if available) - NVIDIA GPUs
//! 2. Metal (if available) - Apple Silicon/AMD GPUs on macOS  
//! 3. CPU (fallback) - Always available

use candle_core::Device;
#[cfg(feature = "cuda")]
use candle_core::utils::cuda_is_available;
#[cfg(feature = "metal")]
use candle_core::utils::metal_is_available;
use log::info;
#[cfg(any(feature = "cuda", feature = "metal"))]
use log::warn;

/// Detects and returns the best available compute device.
///
/// Priority order:
/// 1. CUDA GPU (ordinal 0) if CUDA feature enabled
/// 2. Metal GPU (ordinal 0) if Metal feature enabled
/// 3. CPU as fallback
///
/// # Returns
/// - `Ok(Device)` - Best available device
/// - `Err(_)` - Only if GPU initialization fails (falls back to CPU with warning)
pub fn detect_best_device() -> candle_core::Result<Device> {
    #[cfg(feature = "cuda")]
    {
        if cuda_is_available() {
            match Device::new_cuda(0) {
                Ok(device) => {
                    info!("Using CUDA GPU (device 0) for inference");
                    return Ok(device);
                }
                Err(e) => {
                    warn!(
                        "CUDA available but failed to initialize: {}. Falling back to next option.",
                        e
                    );
                }
            }
        }
    }

    #[cfg(feature = "metal")]
    {
        if metal_is_available() {
            match Device::new_metal(0) {
                Ok(device) => {
                    info!("Using Metal GPU (device 0) for inference");
                    return Ok(device);
                }
                Err(e) => {
                    warn!(
                        "Metal available but failed to initialize: {}. Falling back to CPU.",
                        e
                    );
                }
            }
        }
    }

    info!("Using CPU for inference (no GPU available or GPU initialization failed)");
    Ok(Device::Cpu)
}
