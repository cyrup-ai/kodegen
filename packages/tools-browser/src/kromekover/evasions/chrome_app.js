// Mock Chrome app API
window.chrome = window.chrome || {};
window.chrome.app = {
  InstallState: {
    DISABLED: 'DISABLED',
    INSTALLED: 'INSTALLED',
    NOT_INSTALLED: 'NOT_INSTALLED'
  },
  RunningState: {
    CANNOT_RUN: 'CANNOT_RUN',
    READY_TO_RUN: 'READY_TO_RUN',
    RUNNING: 'RUNNING'
  },
  getDetails: () => {},
  getIsInstalled: () => false,
  installState: () => 'NOT_INSTALLED',
  runningState: () => 'CANNOT_RUN'
};
