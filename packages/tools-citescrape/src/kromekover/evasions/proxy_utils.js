window.utils = window.utils || {};
const utils = window.utils;

utils.stripProxyFromErrors = (handler = {}) => {
  const newHandler = {
    setPrototypeOf: function (target, proto) {
      if (proto === null) throw new TypeError('Cannot convert object to primitive value');
      if (Object.getPrototypeOf(target) === Object.getPrototypeOf(proto)) throw new TypeError('Cyclic __proto__ value');
      return Reflect.setPrototypeOf(target, proto);
    }
  };
  const traps = Object.getOwnPropertyNames(handler);
  traps.forEach(trap => {
    newHandler[trap] = function () {
      try {
        return handler[trap].apply(this, arguments || []);
      } catch (err) {
        if (!err || !err.stack || !err.stack.includes(`at `)) throw err;
        err.stack = err.stack.replace('at Object.toString (', 'at Function.toString (');
        if ((err.stack || '').includes('at Function.toString (')) {
          err.stack = stripWithBlacklist(err.stack, false);
          throw err;
        }
        err.stack = stripWithAnchor(err.stack) || stripWithBlacklist(err.stack);
        throw err;
      }
    };
  });
  function stripWithBlacklist(stack, stripFirstLine = true) {
    const blacklist = [`at Reflect.${trap} `, `at Object.${trap} `, `at Object.newHandler.<computed> [as ${trap}] `];
    return stack.split('\n').filter((line, index) => !(index === 1 && stripFirstLine)).filter(line => !blacklist.some(bl => line.trim().startsWith(bl))).join('\n');
  }
  function stripWithAnchor(stack, anchor) {
    const stackArr = stack.split('\n');
    anchor = anchor || `at Object.newHandler.<computed> [as ${trap}] `;
    const anchorIndex = stackArr.findIndex(line => line.trim().startsWith(anchor));
    if (anchorIndex === -1) return false;
    stackArr.splice(1, anchorIndex);
    return stackArr.join('\n');
  }
  return newHandler;
};

utils.replaceProperty = (obj, propName, descriptorOverrides = {}) => {
  return Object.defineProperty(obj, propName, {
    ...(Object.getOwnPropertyDescriptor(obj, propName) || {}),
    ...descriptorOverrides
  });
};

utils.replaceWithProxy = (obj, propName, handler) => {
  const originalObj = obj[propName];
  const proxyObj = new Proxy(obj[propName], utils.stripProxyFromErrors(handler));
  utils.replaceProperty(obj, propName, { value: proxyObj });
  utils.redirectToString(proxyObj, originalObj);
  return true;
};

utils.replaceGetterWithProxy = (obj, propName, handler) => {
  const fn = Object.getOwnPropertyDescriptor(obj, propName).get;
  const fnStr = fn.toString();
  const proxyObj = new Proxy(fn, utils.stripProxyFromErrors(handler));
  utils.replaceProperty(obj, propName, { get: proxyObj });
  utils.patchToString(proxyObj, fnStr);
  return true;
};

utils.replaceGetterSetter = (obj, propName, handlerGetterSetter) => {
  const ownPropertyDescriptor = Object.getOwnPropertyDescriptor(obj, propName);
  const handler = { ...ownPropertyDescriptor };
  if (handlerGetterSetter.get !== undefined) {
    const nativeFn = ownPropertyDescriptor.get;
    handler.get = function() { return handlerGetterSetter.get.call(this, nativeFn.bind(this)); };
    utils.redirectToString(handler.get, nativeFn);
  }
  if (handlerGetterSetter.set !== undefined) {
    const nativeFn = ownPropertyDescriptor.set;
    handler.set = function(newValue) { handlerGetterSetter.set.call(this, newValue, nativeFn.bind(this)); };
    utils.redirectToString(handler.set, nativeFn);
  }
  Object.defineProperty(obj, propName, handler);
};

utils.mockWithProxy = (obj, propName, pseudoTarget, handler) => {
  const proxyObj = new Proxy(pseudoTarget, utils.stripProxyFromErrors(handler));
  utils.replaceProperty(obj, propName, { value: proxyObj });
  utils.patchToString(proxyObj);
  return true;
};

utils.createProxy = (pseudoTarget, handler) => {
  const proxyObj = new Proxy(pseudoTarget, utils.stripProxyFromErrors(handler));
  utils.patchToString(proxyObj);
  return proxyObj;
};

utils.splitObjPath = objPath => ({
  objName: objPath.split('.').slice(0, -1).join('.'),
  propName: objPath.split('.').slice(-1)[0]
});

utils.replaceObjPathWithProxy = (objPath, handler) => {
  const { objName, propName } = utils.splitObjPath(objPath);
  const obj = eval(objName);
  return utils.replaceWithProxy(obj, propName, handler);
};
