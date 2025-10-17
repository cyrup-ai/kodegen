// Mock navigator language properties
Object.defineProperties(navigator, {
  'language': {
    get: () => 'en-US'
  },
  'languages': {
    get: () => ['en-US', 'en']
  }
});
