//! ImageBuilderImpl struct and ImageBuilder trait implementation

use std::pin::Pin;

use super::api::{ImageBuilder, ResizeFilter};
use super::operations::ImageOperation;
use crate::domain::context::CandleDocumentChunk as ImageChunk;
use crate::domain::image::{ContentFormat, ImageDetail, ImageMediaType};
use tokio_stream::{Stream, StreamExt};

/// Hidden implementation struct - zero-allocation builder state with zero Box<dyn> usage
pub(super) struct ImageBuilderImpl<F1 = fn(String), F2 = fn(ImageChunk) -> ImageChunk>
where
    F1: Fn(String) + Send + Sync + 'static,
    F2: FnMut(ImageChunk) -> ImageChunk + Send + 'static,
{
    pub(super) data: String,
    pub(super) format: Option<ContentFormat>,
    pub(super) media_type: Option<ImageMediaType>,
    pub(super) detail: Option<ImageDetail>,
    pub(super) error_handler: Option<F1>,
    pub(super) chunk_handler: Option<F2>,
    pub(super) operations: Vec<ImageOperation>,
}

impl<F1, F2> ImageBuilder for ImageBuilderImpl<F1, F2>
where
    F1: Fn(String) + Send + Sync + 'static,
    F2: FnMut(ImageChunk) -> ImageChunk + Send + 'static,
{
    /// Set format - EXACT syntax: .format(ContentFormat::Base64)
    fn format(mut self, format: ContentFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set media type - EXACT syntax: .media_type(ImageMediaType::PNG)
    fn media_type(mut self, media_type: ImageMediaType) -> Self {
        self.media_type = Some(media_type);
        self
    }

    /// Set detail - EXACT syntax: .detail(ImageDetail::High)
    fn detail(mut self, detail: ImageDetail) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Set PNG format - EXACT syntax: .with_png()
    fn with_png(mut self) -> Self {
        self.media_type = Some(ImageMediaType::PNG);
        self
    }

    /// Set JPEG format - EXACT syntax: .with_jpeg()
    fn with_jpeg(mut self) -> Self {
        self.media_type = Some(ImageMediaType::JPEG);
        self
    }

    /// Set high detail - EXACT syntax: .high_detail()
    fn high_detail(mut self) -> Self {
        self.detail = Some(ImageDetail::High);
        self
    }

    /// Set low detail - EXACT syntax: .low_detail()
    fn low_detail(mut self) -> Self {
        self.detail = Some(ImageDetail::Low);
        self
    }

    /// Resize image - EXACT syntax: .resize(width, height, filter)
    fn resize(mut self, width: usize, height: usize, filter: ResizeFilter) -> Self {
        self.operations.push(ImageOperation::Resize {
            width,
            height,
            filter,
        });
        self
    }

    /// Normalize to range [-1, 1] - EXACT syntax: .normalize_signed()
    fn normalize_signed(mut self) -> Self {
        self.operations.push(ImageOperation::NormalizeSigned);
        self
    }

    /// Normalize to range [0, 1] - EXACT syntax: .normalize_unsigned()
    fn normalize_unsigned(mut self) -> Self {
        self.operations.push(ImageOperation::NormalizeUnsigned);
        self
    }

    /// Normalize with mean/std per channel - EXACT syntax: .normalize_with(mean, std)
    fn normalize_with(mut self, mean: [f32; 3], std: [f32; 3]) -> Self {
        self.operations
            .push(ImageOperation::NormalizeWithParams { mean, std });
        self
    }

    /// Clamp values to range - EXACT syntax: .clamp(min, max)
    fn clamp(mut self, min: f32, max: f32) -> Self {
        self.operations.push(ImageOperation::Clamp { min, max });
        self
    }

    /// Convert to Candle tensor - EXACT syntax: .to_tensor(device)
    ///
    /// Executes the complete image processing pipeline:
    /// 1. Load image from source (base64/URL/path)
    /// 2. Apply image-level operations (resize, RGB conversion)
    /// 3. Convert image to tensor (HWC→CHW, u8→f32)
    /// 4. Apply tensor-level operations (normalize, clamp)
    /// 5. Transfer to target device
    ///
    /// Returns Future<Output = Result<Tensor, String>> for async execution.
    ///
    /// # Examples
    ///
    /// CLIP-style preprocessing:
    /// ```no_run
    /// # use kodegen_candle_agent::builders::image::{Image, ResizeFilter};
    /// # use candle_core::Device;
    /// # async fn example() -> Result<(), String> {
    /// let device = Device::Cpu;
    /// let tensor = Image::from_path("image.jpg")
    ///     .resize(224, 224, ResizeFilter::Triangle)
    ///     .normalize_signed()
    ///     .to_tensor(&device)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// LLaVA-style preprocessing:
    /// ```no_run
    /// # use kodegen_candle_agent::builders::image::{Image, ResizeFilter};
    /// # use candle_core::Device;
    /// # async fn example() -> Result<(), String> {
    /// let device = Device::Cpu;
    /// let tensor = Image::from_url("https://example.com/image.png")
    ///     .resize(336, 336, ResizeFilter::CatmullRom)
    ///     .normalize_unsigned()
    ///     .normalize_with([0.485, 0.456, 0.406], [0.229, 0.224, 0.225])
    ///     .to_tensor(&device)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn to_tensor(
        self,
        device: &candle_core::Device,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<candle_core::Tensor, String>> + Send + '_>,
    > {
        // Clone device for move into async block
        let device = device.clone();

        Box::pin(async move {
            // Move ALL blocking operations to dedicated blocking thread pool
            // This prevents blocking the tokio runtime for 60-170ms per image
            tokio::task::spawn_blocking(move || {
                // Delegate to synchronous implementation
                self.to_tensor_sync(&device)
            })
            .await
            .map_err(|e| format!("Image processing spawn_blocking failed: {}", e))?
        })
    }

    /// Synchronous tensor creation for use in spawn_blocking contexts
    ///
    /// This method performs the same operations as `to_tensor()` but without
    /// async wrapping, making it suitable for use inside `spawn_blocking`.
    ///
    /// Executes the complete image processing pipeline:
    /// 1. Load image from source (base64/URL/path)
    /// 2. Apply image-level operations (resize, RGB conversion)
    /// 3. Convert image to tensor (HWC→CHW, u8→f32)
    /// 4. Apply tensor-level operations (normalize, clamp)
    /// 5. Transfer to target device
    ///
    /// # Example
    /// ```no_run
    /// # use kodegen_candle_agent::builders::image::{Image, ResizeFilter};
    /// # use candle_core::Device;
    /// # fn example() -> Result<(), String> {
    /// let device = Device::Cpu;
    /// let tensor = Image::from_path("image.jpg")
    ///     .resize(224, 224, ResizeFilter::Triangle)
    ///     .normalize_signed()
    ///     .to_tensor_sync(&device)?;
    /// # Ok(())
    /// # }
    /// ```
    fn to_tensor_sync(self, device: &candle_core::Device) -> Result<candle_core::Tensor, String> {
        // Step 1: Load image from source (base64/URL/path)
        let img = self.load_image_from_source()?;

        // Step 2: Apply image-level operations (resize, RGB conversion)
        let img = self.apply_image_operations(img)?;

        // Step 3: Convert image to tensor (HWC→CHW, u8→f32)
        let tensor = self.image_to_tensor(img)?;

        // Step 4: Apply tensor-level operations (normalize, clamp)
        let tensor = self.apply_tensor_operations(tensor)?;

        // Step 5: Transfer to target device
        let tensor = self.transfer_to_device(tensor, device)?;

        Ok(tensor)
    }

    /// Set error handler - EXACT syntax: .on_error(|error| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_error<F>(self, handler: F) -> impl ImageBuilder
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        ImageBuilderImpl {
            data: self.data,
            format: self.format,
            media_type: self.media_type,
            detail: self.detail,
            error_handler: Some(handler),
            chunk_handler: self.chunk_handler,
            operations: self.operations,
        }
    }

    /// Set chunk handler - EXACT syntax: .on_chunk(|chunk| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_chunk<F>(self, handler: F) -> impl ImageBuilder
    where
        F: FnMut(ImageChunk) -> ImageChunk + Send + 'static,
    {
        ImageBuilderImpl {
            data: self.data,
            format: self.format,
            media_type: self.media_type,
            detail: self.detail,
            error_handler: self.error_handler,
            chunk_handler: Some(handler),
            operations: self.operations,
        }
    }

    /// Load image - EXACT syntax: .load()
    fn load(self) -> Pin<Box<dyn Stream<Item = ImageChunk> + Send>> {
        // Create CandleDocumentChunk with proper fields
        let chunk = ImageChunk {
            path: None,
            content: self.data,
            byte_range: None,
            metadata: std::collections::HashMap::new(),
        };

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let _ = sender.send(chunk);
            },
        ))
    }

    /// Process image - EXACT syntax: .process(|chunk| { ... })
    fn process<F>(self, f: F) -> Pin<Box<dyn Stream<Item = ImageChunk> + Send>>
    where
        F: FnOnce(ImageChunk) -> ImageChunk + Send + 'static,
    {
        // Get source stream from load
        let load_stream = self.load();

        // Create processing stream using async pattern
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            tokio::pin!(load_stream);

            // Process the chunk (FnOnce called once)
            if let Some(chunk) = load_stream.next().await {
                let processed = f(chunk);
                let _ = sender.send(processed);
            }
        }))
    }
}
