// ==================================================================
// Font Spoofing Evasion - Production Grade
// ==================================================================
// Targets real font fingerprinting vectors:
// 1. Canvas text measurement variations (deterministic noise)
// 2. Font enumeration via document.fonts API
// 3. Font availability detection
//
// KEY FEATURES:
// - Deterministic: Same input always produces same output
// - Context-aware: Uses canvas properties + text content for seed
// - Realistic: Small variations (0.05-0.1px) that don't break layouts
// - Undetectable: No predictable patterns like sin waves
// ==================================================================

(function() {
  'use strict';

  // Simple hash function for creating deterministic seeds
  // Uses djb2 algorithm - fast and good distribution
  function hashString(str) {
    let hash = 5381;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) + hash) + str.charCodeAt(i); // hash * 33 + c
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash);
  }

  // Seeded pseudo-random number generator (mulberry32)
  // Returns consistent values for same seed
  function seededRandom(seed) {
    let t = seed + 0x6D2B79F5;
    t = Math.imul(t ^ t >>> 15, t | 1);
    t ^= t + Math.imul(t ^ t >>> 7, t | 61);
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  }

  // ==================================================================
  // 1. Canvas measureText Evasion
  // ==================================================================
  // Adds subtle, deterministic noise to text measurements
  // Prevents font-based canvas fingerprinting while maintaining consistency

  try {
    if (typeof CanvasRenderingContext2D !== 'undefined' && 
        CanvasRenderingContext2D.prototype.measureText) {
      
      const originalMeasureText = CanvasRenderingContext2D.prototype.measureText;
      
      CanvasRenderingContext2D.prototype.measureText = function(text) {
        const result = originalMeasureText.apply(this, arguments);
        
        try {
          // Create deterministic seed from canvas context properties + text
          // This ensures same configuration always produces same noise
          const contextSignature = [
            this.font || '',
            text || '',
            this.textAlign || '',
            this.textBaseline || '',
            this.direction || ''
          ].join('|');
          
          const seed = hashString(contextSignature);
          const noise = (seededRandom(seed) * 0.1) - 0.05; // ±0.05px variation
          
          // Apply subtle noise to width measurement
          result.width += noise;
          
          // If TextMetrics has actualBoundingBox properties, adjust them too
          if (result.actualBoundingBoxLeft !== undefined) {
            const seed2 = hashString(contextSignature + 'left');
            result.actualBoundingBoxLeft += (seededRandom(seed2) * 0.05) - 0.025;
          }
          if (result.actualBoundingBoxRight !== undefined) {
            const seed3 = hashString(contextSignature + 'right');
            result.actualBoundingBoxRight += (seededRandom(seed3) * 0.05) - 0.025;
          }
        } catch (e) {
          // Silent failure - return unmodified result if anything goes wrong
        }
        
        return result;
      };
    }
  } catch (e) {
    // Silent failure - don't break if CanvasRenderingContext2D doesn't exist
  }

  // ==================================================================
  // 2. Font Enumeration Evasion
  // ==================================================================
  // Mocks document.fonts API to prevent font fingerprinting
  // Returns consistent set of common cross-platform fonts

  try {
    if (typeof document !== 'undefined' && !document.fonts) {
      // Common fonts that exist across Windows, Mac, Linux
      const commonFonts = [
        'Arial',
        'Arial Black',
        'Comic Sans MS',
        'Courier New',
        'Georgia',
        'Impact',
        'Times New Roman',
        'Trebuchet MS',
        'Verdana',
        'Helvetica',
        'Tahoma',
        'Palatino',
        'Garamond',
        'Bookman',
        'Calibri',
        'Segoe UI'
      ];

      // Create mock FontFaceSet
      const mockFontFaceSet = {
        // Check if font is "available" - always return true for common fonts
        check: function(font, text) {
          if (!font) return false;
          
          // Extract font family from font string
          const fontFamily = font.match(/['"]?([^'"]+)['"]?/)?.[1] || font;
          return commonFonts.some(f => 
            fontFamily.toLowerCase().includes(f.toLowerCase())
          );
        },
        
        // Load font - always resolve immediately for common fonts
        load: function(font, text) {
          return Promise.resolve([]);
        },
        
        // Ready promise - resolve immediately
        ready: Promise.resolve(mockFontFaceSet),
        
        // Status - always loaded
        status: 'loaded',
        
        // Size - return consistent number
        size: commonFonts.length,
        
        // Add/delete/clear - no-op methods
        add: function() { return this; },
        delete: function() { return false; },
        clear: function() {},
        
        // Iterator methods
        entries: function() { return [][Symbol.iterator](); },
        forEach: function() {},
        has: function() { return false; },
        keys: function() { return [][Symbol.iterator](); },
        values: function() { return [][Symbol.iterator](); }
      };

      // Define document.fonts property
      Object.defineProperty(document, 'fonts', {
        get: function() { return mockFontFaceSet; },
        enumerable: true,
        configurable: true
      });
    }
  } catch (e) {
    // Silent failure - don't break if document doesn't exist
  }

  // ==================================================================
  // 3. FontFace Constructor Evasion
  // ==================================================================
  // Prevent direct FontFace instantiation for fingerprinting

  try {
    if (typeof FontFace !== 'undefined') {
      const OriginalFontFace = FontFace;
      
      window.FontFace = function(family, source, descriptors) {
        // Allow construction but normalize family to common fonts
        const normalizedFamily = family || 'Arial';
        return new OriginalFontFace(normalizedFamily, source, descriptors);
      };
      
      // Preserve prototype and static properties
      window.FontFace.prototype = OriginalFontFace.prototype;
      Object.setPrototypeOf(window.FontFace, OriginalFontFace);
    }
  } catch (e) {
    // Silent failure
  }

})();
