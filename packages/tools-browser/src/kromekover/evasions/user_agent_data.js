// Extract Chrome version from actual user agent
const chromeMatch = navigator.userAgent.match(/Chrome\/(\d+\.\d+\.\d+\.\d+)/);
const chromeVersion = chromeMatch ? chromeMatch[1] : '132.0.6834.160';
const majorVersion = chromeVersion.split('.')[0];

navigator.userAgentData = {
  brands: [
    { brand: 'Google Chrome', version: majorVersion },
    { brand: 'Chromium', version: majorVersion },
    { brand: 'Not=A?Brand', version: '8' }
  ],
  mobile: false,
  platform: 'Windows',
  getHighEntropyValues: async (hints) => {
    const values = {
      architecture: 'x86',
      bitness: '64',
      model: '',
      platformVersion: '10.0.0',
      uaFullVersion: chromeVersion,
      fullVersionList: [
        { brand: 'Google Chrome', version: chromeVersion },
        { brand: 'Chromium', version: chromeVersion }
      ],
      wow64: false
    };
    return hints.reduce((acc, hint) => { 
      acc[hint] = values[hint] || ''; 
      return acc; 
    }, {});
  }
};
