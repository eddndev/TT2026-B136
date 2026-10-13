const names = [
  'principalId',
  'contextId',
  'editorKind',
  'resourceId',
  'action',
  'instanceId',
  'ownerDraftKey',
  'fieldPath',
  'rowId',
  'schemaVersion',
  'baseRevision',
];

export function plainRecord(value) {
  if (!value || typeof value !== 'object') return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

export function nonemptyString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

export function copyDescriptor(value) {
  return { ...value, fieldPath: [...value.fieldPath] };
}

export function validateDescriptor(value, principalId, lookup) {
  if (!plainRecord(value) || Reflect.ownKeys(value).length !== names.length)
    throw new TypeError('Invalid draft descriptor.');
  const descriptor = {};
  for (const name of names) {
    const property = Object.getOwnPropertyDescriptor(value, name);
    if (!property || !Object.hasOwn(property, 'value'))
      throw new TypeError('Invalid draft descriptor field.');
    descriptor[name] = property.value;
  }
  const required = ['principalId', 'contextId', 'editorKind', 'action'];
  const optional = ['resourceId', 'instanceId', 'ownerDraftKey', 'rowId'];
  if (
    required.some((name) => !nonemptyString(descriptor[name])) ||
    optional.some((name) => descriptor[name] !== null && !nonemptyString(descriptor[name])) ||
    descriptor.principalId !== principalId ||
    !Number.isSafeInteger(descriptor.schemaVersion) ||
    descriptor.schemaVersion < 1 ||
    (descriptor.baseRevision !== null &&
      (!Number.isSafeInteger(descriptor.baseRevision) || descriptor.baseRevision < 0)) ||
    !Array.isArray(descriptor.fieldPath) ||
    [...descriptor.fieldPath].some((part) => !nonemptyString(part))
  )
    throw new TypeError('Invalid draft descriptor value.');

  if (descriptor.ownerDraftKey === null) {
    if (descriptor.fieldPath.length || descriptor.rowId !== null)
      throw new TypeError('A nested draft requires an owner.');
  } else {
    const parent = lookup(descriptor.ownerDraftKey);
    if (
      !parent ||
      !descriptor.fieldPath.length ||
      parent.principalId !== descriptor.principalId ||
      parent.contextId !== descriptor.contextId
    )
      throw new TypeError('Invalid draft owner.');
  }
  descriptor.fieldPath = Object.freeze([...descriptor.fieldPath]);
  return Object.freeze(descriptor);
}

export function descriptorKey(value) {
  // Revisions describe the saved base; they do not identify a new editor.
  return JSON.stringify([
    value.principalId,
    value.contextId,
    value.editorKind,
    value.resourceId,
    value.action,
    value.instanceId,
    value.ownerDraftKey,
    value.fieldPath,
    value.rowId,
  ]);
}
