import { nonemptyString, plainRecord } from './draft-descriptor.mjs';

const excludedNames = new Set([
  'accesstoken',
  'password',
  'challengetoken',
  'totpsecret',
  'totpsecretbase32',
  'otpauthuri',
  'recoverycodes',
  'authorization',
  'api',
  'client',
  'callback',
  'callbacks',
  'busy',
  'promise',
  'promises',
  '__proto__',
  'constructor',
  'prototype',
]);

function safeName(name) {
  return (
    nonemptyString(name) &&
    !excludedNames.has(name.toLowerCase().replace(/[_-]/g, '')) &&
    !['__proto__', 'constructor', 'prototype'].includes(name)
  );
}

export function validateFields(fields) {
  if (
    !Array.isArray(fields) ||
    !fields.length ||
    [...fields].some((name) => !safeName(name)) ||
    new Set(fields).size !== fields.length
  )
    throw new TypeError('Draft fields must be an explicit unique allowlist.');
  return [...fields];
}

export function discardAsyncResult(value) {
  if (value instanceof Promise) Promise.prototype.then.call(value, undefined, () => {});
}

export function asynchronous(value) {
  if (value === null || !['object', 'function'].includes(typeof value)) return false;
  for (let current = value; current !== null; current = Object.getPrototypeOf(current)) {
    const property = Object.getOwnPropertyDescriptor(current, 'then');
    if (property) return !Object.hasOwn(property, 'value') || typeof property.value === 'function';
  }
  return false;
}

export function cloneDraftValue(value, ancestors = new Set()) {
  if (value === null || ['undefined', 'string', 'number', 'boolean'].includes(typeof value))
    return value;
  if (typeof value !== 'object' || ancestors.has(value))
    throw new TypeError('Draft values must contain only acyclic data.');

  if (typeof File !== 'undefined' && Object.getPrototypeOf(value) === File.prototype) {
    if (Object.getOwnPropertyNames(value).length)
      throw new TypeError('File draft values cannot contain custom fields.');
    return new File([value], value.name, { type: value.type, lastModified: value.lastModified });
  }
  if (typeof Blob !== 'undefined' && Object.getPrototypeOf(value) === Blob.prototype) {
    if (Object.getOwnPropertyNames(value).length)
      throw new TypeError('Blob draft values cannot contain custom fields.');
    return new Blob([value], { type: value.type });
  }

  const array = Array.isArray(value);
  if (
    (array && Object.getPrototypeOf(value) !== Array.prototype) ||
    (!array && !plainRecord(value))
  )
    throw new TypeError('Runtime objects are not draft values.');
  ancestors.add(value);
  try {
    const result = array ? new Array(value.length) : {};
    for (const name of Reflect.ownKeys(value)) {
      if (array && name === 'length') continue;
      const property = Object.getOwnPropertyDescriptor(value, name);
      if (!safeName(name) || !property.enumerable || !Object.hasOwn(property, 'value'))
        throw new TypeError('Draft values contain an unsupported field.');
      result[name] = cloneDraftValue(property.value, ancestors);
    }
    return result;
  } finally {
    ancestors.delete(value);
  }
}

export function captureProjection(capture, fields) {
  const value = capture();
  discardAsyncResult(value);
  if (!plainRecord(value) || asynchronous(value))
    throw new TypeError('Draft capture must return synchronous fields.');
  const result = {};
  for (const name of fields) {
    const property = Object.getOwnPropertyDescriptor(value, name);
    if (!property || !Object.hasOwn(property, 'value'))
      throw new TypeError('A projected draft field is missing or computed.');
    result[name] = cloneDraftValue(property.value);
  }
  return result;
}
