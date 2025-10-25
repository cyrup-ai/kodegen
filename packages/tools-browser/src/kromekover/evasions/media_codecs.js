// Media Capabilities API evasion with full stealth
// Uses utils.replaceWithProxy() for undetectable function replacement
// Reference: https://developer.mozilla.org/en-US/docs/Web/API/MediaCapabilities/decodingInfo

(() => {
  // Guard: Only patch if MediaCapabilities exists and has decodingInfo
  if (!navigator.mediaCapabilities || 
      typeof navigator.mediaCapabilities.decodingInfo !== 'function') {
    return;
  }

  // Store original function
  const originalDecodingInfo = navigator.mediaCapabilities.decodingInfo;

  // Comprehensive codec support matching modern browsers
  const VIDEO_CODECS = [
    'vp8', 'vp9', 'vp09',           // VP8, VP9
    'av01',                          // AV1
    'avc1', 'avc3', 'h264',         // H.264/AVC
    'hev1', 'hvc1', 'h265',         // H.265/HEVC
    'mp4v'                           // MPEG-4 Visual
  ];

  const AUDIO_CODECS = [
    'opus',                          // Opus
    'vorbis',                        // Vorbis
    'mp4a',                          // AAC
    'mp3',                           // MP3
    'flac',                          // FLAC
    'pcm'                            // PCM
  ];

  // Validation helper matching native behavior
  function validateConfig(config) {
    if (!config || typeof config !== 'object') {
      throw new TypeError(
        "Failed to execute 'decodingInfo' on 'MediaCapabilities': " +
        "1 argument required, but only 0 present."
      );
    }

    if (!config.type || !['file', 'media-source', 'webrtc'].includes(config.type)) {
      throw new TypeError(
        "Failed to execute 'decodingInfo' on 'MediaCapabilities': " +
        "The provided value '" + config.type + "' is not a valid enum value of type MediaDecodingType."
      );
    }

    if (!config.video && !config.audio) {
      throw new TypeError(
        "Failed to execute 'decodingInfo' on 'MediaCapabilities': " +
        "The configuration must have an audio or a video field."
      );
    }
  }

  // Check if codec should be reported as supported
  function shouldSupportCodec(contentType, codecList) {
    if (!contentType || typeof contentType !== 'string') {
      return false;
    }
    return codecList.some(codec => contentType.toLowerCase().includes(codec));
  }

  // Create stealth wrapper function
  const stealthDecodingInfo = function(config) {
    // Validate inputs like native implementation
    validateConfig(config);

    // Call original implementation
    return originalDecodingInfo.call(this, config)
      .then(result => {
        // Check video codecs
        if (config.video && config.video.contentType) {
          if (shouldSupportCodec(config.video.contentType, VIDEO_CODECS)) {
            return {
              ...result,  // Don't mutate original
              supported: true,
              smooth: true,
              powerEfficient: true
            };
          }
        }

        // Check audio codecs
        if (config.audio && config.audio.contentType) {
          if (shouldSupportCodec(config.audio.contentType, AUDIO_CODECS)) {
            return {
              ...result,  // Don't mutate original
              supported: true,
              smooth: true,
              powerEfficient: true  // Per spec: all audio codecs report true
            };
          }
        }

        return result;
      })
      .catch(err => {
        // Pass through native errors without exposing our wrapper
        throw err;
      });
  };

  // STEALTH: Use utils.replaceWithProxy for undetectable patching
  utils.replaceWithProxy(
    navigator.mediaCapabilities,
    'decodingInfo',
    {
      apply(target, thisArg, args) {
        return stealthDecodingInfo.apply(thisArg, args);
      }
    }
  );

  // STEALTH: Make function.toString() return native-looking code
  utils.patchToString(
    navigator.mediaCapabilities.decodingInfo,
    utils.makeNativeString('decodingInfo')
  );
})();
