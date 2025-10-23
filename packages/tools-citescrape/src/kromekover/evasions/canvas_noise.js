// ==================================================================
// Canvas Fingerprinting Protection - Bit-Flipping Algorithm
// ==================================================================
// Based on research from:
// - Brave Browser farbling (per-session, per-domain)
// - CanvasBlocker extension (bit-flipping technique)
// ==================================================================

(function() {
  'use strict';

  // ====== UTILITY FUNCTIONS ======

  // djb2 hash algorithm for seed generation
  function hashString(str) {
    let hash = 5381;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) + hash) + str.charCodeAt(i);
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash);
  }

  // Generate 128-byte persistent seed for this canvas
  function generatePersistentSeed(canvas) {
    const sessionSeed = window.grokConfig?.sessionSeed || 'default';
    const signature = [
      sessionSeed,
      canvas.width || 0,
      canvas.height || 0
    ].join('|');
    
    // Generate 128 bytes deterministically
    const seed = new Uint8Array(128);
    for (let i = 0; i < 128; i++) {
      seed[i] = hashString(signature + '|' + i) & 0xFF;
    }
    return seed;
  }

  // ====== BIT-FLIPPING RNG (CanvasBlocker Algorithm) ======

  function getBitRng(seed) {
    // seed is Uint8Array(128)
    return function(value, i) {
      // Use last 7 bits from value for byte index (0-127)
      const byteIndex = value & 0x7F;
      
      // Use position bits to get bit index (0-7)
      const bitIndex = ((i & 0x03) << 1) | (value >>> 7);
      
      // Extract the bit from seed
      const bit = (seed[byteIndex] >>> bitIndex) & 0x01;
      
      return bit;
    };
  }

  function getValueRng(seed) {
    const bitRng = getBitRng(seed);
    
    return function(value, i) {
      const bit = bitRng(value, i);
      
      // XOR the last bit to flip it... or not
      // This is the KEY: only ±1 change, minimal noise!
      return value ^ (bit & 0x01);
    };
  }

  function getPixelRng(seed, ignoredColors) {
    const valueRng = getValueRng(seed);
    
    // eslint-disable-next-line max-params
    return function(r, g, b, a, i) {
      // Check if this color should be ignored
      const colorKey = String.fromCharCode(r, g, b, a);
      if (ignoredColors && ignoredColors[colorKey]) {
        return [r, g, b, a];
      }
      
      const baseIndex = i * 4;
      return [
        valueRng(r, baseIndex + 0),
        valueRng(g, baseIndex + 1),
        valueRng(b, baseIndex + 2),
        valueRng(a, baseIndex + 3)
      ];
    };
  }

  // ====== CANVAS GETIMAGEDATA OVERRIDE ======

  try {
    if (typeof CanvasRenderingContext2D === 'undefined' ||
        !CanvasRenderingContext2D.prototype.getImageData) {
      return; // Canvas API not available
    }

    const originalGetImageData = CanvasRenderingContext2D.prototype.getImageData;
    
    CanvasRenderingContext2D.prototype.getImageData = function(...args) {
      const imageData = originalGetImageData.apply(this, arguments);
      
      try {
        // Skip empty canvases
        if (!this.canvas || 
            (this.canvas.width || 0) * (this.canvas.height || 0) === 0) {
          return imageData;
        }

        // Generate seed for this canvas
        const seed = generatePersistentSeed(this.canvas);
        
        // Create pixel RNG (no ignored colors for now)
        const pixelRng = getPixelRng(seed, {});
        
        // Apply bit-flipping to each pixel
        const data = imageData.data;
        const length = data.length;
        
        for (let i = 0; i < length; i += 4) {
          const pixelIndex = i / 4;
          const [r, g, b, a] = pixelRng(
            data[i + 0],
            data[i + 1],
            data[i + 2],
            data[i + 3],
            pixelIndex
          );
          
          data[i + 0] = r;
          data[i + 1] = g;
          data[i + 2] = b;
          data[i + 3] = a;
        }
      } catch (e) {
        // Silent failure - return unmodified data
      }
      
      return imageData;
    };

    // Ensure toString() looks native
    try {
      Object.defineProperty(CanvasRenderingContext2D.prototype.getImageData, 'toString', {
        value: function() {
          return 'function getImageData() { [native code] }';
        },
        writable: false,
        configurable: true
      });
    } catch (e) {
      // Silent failure
    }

  } catch (e) {
    // Silent failure - don't break if CanvasRenderingContext2D doesn't exist
  }

  // ====== CANVAS TODATAURL OVERRIDE ======

  try {
    if (typeof HTMLCanvasElement === 'undefined' ||
        !HTMLCanvasElement.prototype.toDataURL) {
      return;
    }

    const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
    
    HTMLCanvasElement.prototype.toDataURL = function(...args) {
      try {
        // Skip empty canvases
        if ((this.width || 0) * (this.height || 0) === 0) {
          return originalToDataURL.apply(this, arguments);
        }

        // Get context and imageData (this will apply getImageData modifications)
        const ctx = this.getContext('2d');
        if (ctx) {
          // Reading via getImageData applies our modifications
          const imageData = ctx.getImageData(0, 0, this.width, this.height);
          
          // Create new canvas with modified data
          const tempCanvas = document.createElement('canvas');
          tempCanvas.width = this.width;
          tempCanvas.height = this.height;
          const tempCtx = tempCanvas.getContext('2d');
          
          // Use original getImageData to avoid double-modification
          tempCtx.putImageData(imageData, 0, 0);
          
          return originalToDataURL.apply(tempCanvas, arguments);
        }
      } catch (e) {
        // Silent failure
      }
      
      return originalToDataURL.apply(this, arguments);
    };

    // Ensure toString() looks native
    try {
      Object.defineProperty(HTMLCanvasElement.prototype.toDataURL, 'toString', {
        value: function() {
          return 'function toDataURL() { [native code] }';
        },
        writable: false,
        configurable: true
      });
    } catch (e) {
      // Silent failure
    }

  } catch (e) {
    // Silent failure
  }

})();
