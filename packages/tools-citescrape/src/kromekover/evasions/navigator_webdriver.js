// Hide navigator.webdriver flag
Object.defineProperty(navigator, 'webdriver', {
  get: () => false
});
