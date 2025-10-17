window.utils = window.utils || {};
const utils = window.utils;

utils.init = () => {
  utils.preloadCache();
};

utils.preloadCache = () => {
  if (utils.cache) return;
  utils.cache = {
    Reflect: {
      get: Reflect.get.bind(Reflect),
      apply: Reflect.apply.bind(Reflect)
    },
    nativeToStringStr: Function.toString + ''
  };
};

utils.makeNativeString = (name = '') => {
  return utils.cache.nativeToStringStr.replace('toString', name || '');
};

utils.patchToString = (obj, str = '') => {
  const handler = {
    apply: function (target, ctx) {
      if (ctx === Function.prototype.toString) return utils.makeNativeString('toString');
      if (ctx === obj) return str || utils.makeNativeString(obj.name);
      const hasSameProto = Object.getPrototypeOf(Function.prototype.toString).isPrototypeOf(ctx.toString);
      if (!hasSameProto) return ctx.toString();
      return target.call(ctx);
    }
  };
  const toStringProxy = new Proxy(Function.prototype.toString, utils.stripProxyFromErrors(handler));
  utils.replaceProperty(Function.prototype, 'toString', { value: toStringProxy });
};

utils.patchToStringNested = (obj = {}) => {
  return utils.execRecursively(obj, ['function'], utils.patchToString);
};

utils.redirectToString = (proxyObj, originalObj) => {
  const handler = {
    apply: function (target, ctx) {
      if (ctx === Function.prototype.toString) return utils.makeNativeString('toString');
      if (ctx === proxyObj) {
        const fallback = () => originalObj && originalObj.name ? utils.makeNativeString(originalObj.name) : utils.makeNativeString(proxyObj.name);
        return originalObj + '' || fallback();
      }
      if (typeof ctx === 'undefined' || ctx === null) return target.call(ctx);
      const hasSameProto = Object.getPrototypeOf(Function.prototype.toString).isPrototypeOf(ctx.toString);
      if (!hasSameProto) return ctx.toString();
      return target.call(ctx);
    }
  };
  const toStringProxy = new Proxy(Function.prototype.toString, utils.stripProxyFromErrors(handler));
  utils.replaceProperty(Function.prototype, 'toString', { value: toStringProxy });
};

utils.execRecursively = (obj = {}, typeFilter = [], fn) => {
  function recurse(obj) {
    for (const key in obj) {
      if (obj[key] === undefined) continue;
      if (obj[key] && typeof obj[key] === 'object') recurse(obj[key]);
      else if (obj[key] && typeFilter.includes(typeof obj[key])) fn.call(this, obj[key]);
    }
  }
  recurse(obj);
  return obj;
};

utils.stringifyFns = (fnObj = { hello: () => 'world' }) => {
  function fromEntries(iterable) {
    return [...iterable].reduce((obj, [key, val]) => { obj[key] = val; return obj; }, {});
  }
  return (Object.fromEntries || fromEntries)(
    Object.entries(fnObj).filter(([key, value]) => typeof value === 'function').map(([key, value]) => [key, value.toString()])
  );
};

utils.materializeFns = (fnStrObj = { hello: "() => 'world'" }) => {
  return Object.fromEntries(
    Object.entries(fnStrObj).map(([key, value]) => {
      if (value.startsWith('function')) return [key, eval(`() => ${value}`)()];
      return [key, eval(value)];
    })
  );
};

utils.arrayEquals = (array1, array2) => {
  if (array1.length !== array2.length) return false;
  for (let i = 0; i < array1.length; ++i) if (array1[i] !== array2[i]) return false;
  return true;
};

utils.memoize = fn => {
  const cache = [];
  return function(...args) {
    if (!cache.some(c => utils.arrayEquals(c.key, args))) cache.push({ key: args, value: fn.apply(this, args) });
    return cache.find(c => utils.arrayEquals(c.key, args)).value;
  };
};

utils.makeHandler = () => ({
  getterValue: value => ({
    apply(target, ctx, args) {
      utils.cache.Reflect.apply(...arguments);
      return value;
    }
  })
});

utils.init();
