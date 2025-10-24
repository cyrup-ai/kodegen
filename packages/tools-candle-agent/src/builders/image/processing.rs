//! Private image processing pipeline methods

use super::builder_impl::ImageBuilderImpl;
use super::operations::{ImageOperation, convert_filter};
use crate::domain::context::CandleDocumentChunk as ImageChunk;
use crate::domain::image::ContentFormat;
use base64::Engine;
use candle_core::{DType, Device, Tensor};
use image::{DynamicImage, ImageReader};

// Private helper methods for image loading and processing
impl<F1, F2> ImageBuilderImpl<F1, F2>
where
    F1: Fn(String) + Send + Sync + 'static,
    F2: FnMut(ImageChunk) -> ImageChunk + Send + 'static,
{
    /// Load image from data based on format
    ///
    /// Supports three loading modes:
    /// - Base64: Decode base64 string → bytes → DynamicImage
    /// - Url: Load from HTTP URL (treated as file path by ImageReader)
    /// - Raw: Load from file path
    ///
    /// Pattern references:
    /// - File loading: tmp/candle-examples/candle-examples/examples/clip/main.rs:35
    /// - Base64: Standard Rust pattern with image::load_from_memory
    pub(super) fn load_image_from_source(&self) -> Result<DynamicImage, String> {
        match &self.format {
            Some(ContentFormat::Url) | Some(ContentFormat::Raw) => {
                // File path or URL loading (CLIP pattern)
                ImageReader::open(&self.data)
                    .map_err(|e| format!("Failed to open image: {}", e))?
                    .decode()
                    .map_err(|e| format!("Failed to decode image: {}", e))
            }
            Some(ContentFormat::Base64) => {
                // Base64 decoding pattern
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&self.data)
                    .map_err(|e| format!("Failed to decode base64: {}", e))?;
                image::load_from_memory(&bytes)
                    .map_err(|e| format!("Failed to load image from memory: {}", e))
            }
            None => Err("No format specified".to_string()),
        }
    }

    /// Apply image-level operations (resize, RGB conversion)
    ///
    /// This processes operations that work on DynamicImage before tensor conversion:
    /// - Resize: Change image dimensions with specified filter
    /// - ToRGB: Convert to RGB8 format for consistency
    ///
    /// Skips tensor-level operations (normalization, clamp, permute) - those are
    /// handled in IMG_4B during tensor conversion.
    ///
    /// Uses resize_exact() to match user-specified dimensions precisely.
    ///
    /// Pattern references:
    /// - CLIP resize: tmp/candle-examples/candle-examples/examples/clip/main.rs:38-42
    /// - RGB conversion: tmp/candle-examples/candle-examples/examples/clip/main.rs:44
    pub(super) fn apply_image_operations(
        &self,
        mut img: DynamicImage,
    ) -> Result<DynamicImage, String> {
        for op in &self.operations {
            img = match op {
                ImageOperation::Resize {
                    width,
                    height,
                    filter,
                } => {
                    let filter_type = convert_filter(*filter);
                    // Use resize_exact for precise user-specified dimensions
                    // This ensures width × height exactly, without aspect ratio preservation
                    img.resize_exact(*width as u32, *height as u32, filter_type)
                }
                // Skip tensor operations - handled in apply_tensor_operations
                ImageOperation::NormalizeSigned
                | ImageOperation::NormalizeUnsigned
                | ImageOperation::NormalizeWithParams { .. }
                | ImageOperation::Clamp { .. } => img,
            };
        }
        Ok(img)
    }

    /// Convert RGB8 image to Candle tensor in CHW format
    ///
    /// Steps:
    /// 1. Convert to RGB8 format (ensures 3 channels, u8 values)
    /// 2. Extract raw pixel data as Vec<u8>
    /// 3. Create tensor in HWC format (Height, Width, Channel)
    /// 4. Permute to CHW format (Channel, Height, Width) - Candle's native format
    /// 5. Convert from u8 to f32 dtype for processing
    ///
    /// Pattern references:
    /// - CLIP: tmp/candle-examples/candle-examples/examples/clip/main.rs:44-47
    /// - LLaVA: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:138-140
    pub(super) fn image_to_tensor(&self, img: DynamicImage) -> Result<Tensor, String> {
        // Step 1: Convert to RGB8 (3 channels, u8 values)
        let img = img.to_rgb8();

        // Step 2: Get dimensions
        let (width, height) = img.dimensions();

        // Step 3: Extract raw pixel data
        let data = img.into_raw();

        // Step 4: Create tensor in HWC format (Height, Width, Channel)
        let tensor = Tensor::from_vec(data, (height as usize, width as usize, 3), &Device::Cpu)
            .map_err(|e| format!("Failed to create tensor: {}", e))?;

        // Step 5: Permute to CHW format (Channel, Height, Width)
        // This is Candle's native format for vision models
        let tensor = tensor
            .permute((2, 0, 1))
            .map_err(|e| format!("Failed to permute tensor: {}", e))?;

        // Step 6: Convert to f32 for processing
        let tensor = tensor
            .to_dtype(DType::F32)
            .map_err(|e| format!("Failed to convert dtype: {}", e))?;

        Ok(tensor)
    }

    /// Apply tensor-level operations (normalize, clamp)
    ///
    /// Processes tensor operations from the operations queue:
    /// - NormalizeSigned: [0, 255] → [-1, 1] using affine(2/255, -1)
    /// - NormalizeUnsigned: [0, 255] → [0, 1] using division
    /// - NormalizeWithParams: (x/255 - mean) / std for per-channel normalization
    /// - Clamp: Constrain values to [min, max] range
    ///
    /// Pattern references:
    /// - CLIP affine: tmp/candle-examples/candle-examples/examples/clip/main.rs:48
    /// - LLaVA rescale: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:142
    /// - LLaVA normalize: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:146-151
    pub(super) fn apply_tensor_operations(&self, mut tensor: Tensor) -> Result<Tensor, String> {
        for op in &self.operations {
            tensor = match op {
                ImageOperation::NormalizeSigned => {
                    // CLIP-style: [0, 255] → [-1, 1]
                    // Formula: (x * 2/255) - 1 = x * (2/255) + (-1)
                    // Reference: tmp/candle-examples/candle-examples/examples/clip/main.rs:46
                    tensor
                        .affine(2.0 / 255.0, -1.0)
                        .map_err(|e| format!("Signed normalization failed: {}", e))?
                }

                ImageOperation::NormalizeUnsigned => {
                    // Simple scaling: [0, 255] → [0, 1]
                    // Formula: x / 255 = x * (1/255) + 0
                    // Reference: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:141-143
                    tensor
                        .affine(1.0 / 255.0, 0.0)
                        .map_err(|e| format!("Unsigned normalization failed: {}", e))?
                }

                ImageOperation::NormalizeWithParams { mean, std } => {
                    // LLaVA/ImageNet-style: (x - mean) / std
                    // Reference: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:146-150

                    // Step 1: Scale to [0,1] using affine
                    let normalized = tensor
                        .affine(1.0 / 255.0, 0.0)
                        .map_err(|e| format!("Pre-normalization scaling failed: {}", e))?;

                    // Step 2: Create mean tensor - shape (3,) for broadcasting
                    let mean_tensor = Tensor::from_vec(mean.to_vec(), (3,), &Device::Cpu)
                        .map_err(|e| format!("Failed to create mean tensor: {}", e))?;

                    // Step 3: Create std tensor - shape (3,) for broadcasting
                    let std_tensor = Tensor::from_vec(std.to_vec(), (3,), &Device::Cpu)
                        .map_err(|e| format!("Failed to create std tensor: {}", e))?;

                    // Step 4: Apply (x - mean) / std using broadcast operations
                    // broadcast_sub and broadcast_div handle shape alignment automatically
                    let subtracted = normalized
                        .broadcast_sub(&mean_tensor)
                        .map_err(|e| format!("Mean subtraction failed: {}", e))?;

                    subtracted
                        .broadcast_div(&std_tensor)
                        .map_err(|e| format!("Std division failed: {}", e))?
                }

                ImageOperation::Clamp { min, max } => {
                    // Restrict values to [min, max] range
                    tensor
                        .clamp(*min as f64, *max as f64)
                        .map_err(|e| format!("Clamp failed: {}", e))?
                }

                // Skip image-level operations - handled in IMG_4A
                _ => tensor,
            };
        }
        Ok(tensor)
    }

    /// Transfer tensor to target device
    ///
    /// Moves tensor from CPU to the specified device (GPU/Metal/CPU).
    /// Initial processing is done on CPU, final tensor is on target device.
    ///
    /// Device::clone() is cheap (Arc pointer), safe for frequent use.
    pub(super) fn transfer_to_device(
        &self,
        tensor: Tensor,
        device: &Device,
    ) -> Result<Tensor, String> {
        tensor
            .to_device(device)
            .map_err(|e| format!("Failed to transfer to device: {}", e))
    }
}
