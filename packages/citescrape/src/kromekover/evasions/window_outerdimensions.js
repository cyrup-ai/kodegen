// Store original values
const actualInnerWidth = window.innerWidth;
const actualInnerHeight = window.innerHeight;

// Add realistic chrome
Object.defineProperties(window, {
  'outerWidth': { get: () => actualInnerWidth + 16 },
  'outerHeight': { get: () => actualInnerHeight + 135 },
  'innerWidth': { get: () => actualInnerWidth },
  'innerHeight': { get: () => actualInnerHeight }
});
