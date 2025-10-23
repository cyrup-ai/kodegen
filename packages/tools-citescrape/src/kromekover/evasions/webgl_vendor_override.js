// Override WebGL vendor and renderer to match platform configuration
// Uses values from window.grokConfig (injected by mod.rs before this script runs)

// Get config values with fallbacks matching config.rs defaults
const WEBGL_VENDOR = (window.grokConfig && window.grokConfig.webglVendor) || 'Intel Inc.';
const WEBGL_RENDERER = (window.grokConfig && window.grokConfig.webglRenderer) || 'Intel(R) UHD Graphics';

// Store original getParameter method
const originalGetParameter = WebGLRenderingContext.prototype.getParameter;

// Create patched getParameter function
const patchedGetParameter = function(parameter) {
  // 37445 = UNMASKED_VENDOR_WEBGL
  if (parameter === 37445) {
    return WEBGL_VENDOR;
  }
  // 37446 = UNMASKED_RENDERER_WEBGL
  if (parameter === 37446) {
    return WEBGL_RENDERER;
  }
  return originalGetParameter.apply(this, arguments);
};

// Patch WebGLRenderingContext (WebGL 1.0)
WebGLRenderingContext.prototype.getParameter = patchedGetParameter;

// Patch WebGL2RenderingContext (WebGL 2.0) if available
if (typeof WebGL2RenderingContext !== 'undefined') {
  WebGL2RenderingContext.prototype.getParameter = patchedGetParameter;
}
