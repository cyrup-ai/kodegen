// Mock Chrome runtime API with defensive pattern
window.chrome = window.chrome || {};
window.chrome.runtime = {
  connect: () => ({
    onMessage: {
      addListener: () => {},
      removeListener: () => {}
    },
    postMessage: () => {},
    disconnect: () => {}
  }),
  sendMessage: () => {},
  onMessage: {
    addListener: () => {},
    removeListener: () => {}
  }
};

// Link navigator.chrome to window.chrome (must be same object reference)
Object.defineProperty(navigator, 'chrome', {
  get: () => window.chrome,
  configurable: true
});