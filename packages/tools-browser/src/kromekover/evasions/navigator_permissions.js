(() => {
  // Only override if navigator.permissions doesn't exist or is broken
  // Modern Chrome has a working permissions API, so we should be careful
  const hasWorkingPermissions = 'permissions' in navigator && 
                                typeof navigator.permissions.query === 'function';
  
  if (hasWorkingPermissions) {
    return; // Native API works, leave it alone
  }

  // Realistic permission states - most should be 'prompt' or 'denied', not 'granted'
  // Granting everything is a detection vector
  const permissionStates = {
    'geolocation': 'prompt',
    'notifications': 'prompt',
    'push': 'prompt',
    'midi': 'prompt',
    'camera': 'prompt',
    'microphone': 'prompt',
    'speaker': 'prompt',
    'device-info': 'granted',
    'background-sync': 'granted',
    'bluetooth': 'prompt',
    'persistent-storage': 'prompt',
    'ambient-light-sensor': 'prompt',
    'accelerometer': 'prompt',
    'gyroscope': 'prompt',
    'magnetometer': 'prompt',
    'clipboard-read': 'prompt',
    'clipboard-write': 'prompt',
    'payment-handler': 'prompt',
    'idle-detection': 'prompt',
    'periodic-background-sync': 'prompt',
    'screen-wake-lock': 'prompt',
    'nfc': 'prompt',
    'storage-access': 'prompt',
    'window-placement': 'prompt'
  };

  // Create a proper PermissionStatus object
  class PermissionStatus extends EventTarget {
    constructor(state) {
      super();
      this._state = state;
      this._onchange = null;
    }

    get state() {
      return this._state;
    }

    get onchange() {
      return this._onchange;
    }

    set onchange(handler) {
      if (this._onchange) {
        this.removeEventListener('change', this._onchange);
      }
      this._onchange = handler;
      if (handler) {
        this.addEventListener('change', handler);
      }
    }

    // Note: Real PermissionStatus doesn't allow state changes from outside
    // But we include internal method for potential future use
    _updateState(newState) {
      if (this._state !== newState) {
        this._state = newState;
        this.dispatchEvent(new Event('change'));
      }
    }
  }

  // Make PermissionStatus.prototype.constructor look native
  utils.patchToString(PermissionStatus, utils.makeNativeString('PermissionStatus'));
  utils.patchToString(PermissionStatus.prototype.constructor, utils.makeNativeString('PermissionStatus'));

  // Create the Permissions object
  const permissions = {
    query: async function(permissionDesc) {
      if (!permissionDesc || typeof permissionDesc !== 'object') {
        return Promise.reject(new TypeError(
          "Failed to execute 'query' on 'Permissions': 1 argument required, but only 0 present."
        ));
      }

      const name = permissionDesc.name;
      if (!name) {
        return Promise.reject(new TypeError(
          "Failed to execute 'query' on 'Permissions': required member name is undefined."
        ));
      }

      // Get state for this permission, default to 'prompt'
      const state = permissionStates[name] || 'prompt';
      
      // Return a proper PermissionStatus object
      return new PermissionStatus(state);
    }
  };

  // Make the query function look native
  utils.patchToString(permissions.query, utils.makeNativeString('query'));

  // Use utils.replaceProperty to set navigator.permissions
  // This is more stealthy than direct assignment
  utils.replaceProperty(Object.getPrototypeOf(navigator), 'permissions', {
    get() {
      return permissions;
    }
  });

  // Ensure the getter itself looks native
  const descriptor = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(navigator), 'permissions');
  if (descriptor && descriptor.get) {
    utils.patchToString(descriptor.get, utils.makeNativeString('get permissions'));
  }
})();
