import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factText as text,
} from './procedural-fact-primitives.mjs';

const canonical = (value, parse) => {
  if (parse(value) !== value) invalid('La referencia debe conservar su forma exacta.');
  return value;
};
export function resourceActivityReference(value, act = false) {
  object(value, ['id', 'revision', 'capture_digest', ...(act ? ['resource_revision'] : [])]);
  canonical(value.id, uuid);
  revision(value.revision);
  canonical(value.capture_digest, digest);
  if (act) revision(value.resource_revision);
  return structuredClone(value);
}
export function resourceActivityTarget(value) {
  if (!['hearing', 'deadline'].includes(value?.kind)) invalid();
  const hash = value.kind === 'hearing' ? 'submission_digest' : 'capture_digest';
  object(value, ['kind', 'id', 'revision', hash]);
  canonical(value.id, uuid);
  revision(value.revision);
  canonical(value[hash], digest);
  return structuredClone(value);
}
export function resourceActivitySelection(value, resourceId, headRevision) {
  object(value, ['resource', 'act', 'target']);
  const resource = resourceActivityReference(value.resource);
  const act = value.act === null ? null : resourceActivityReference(value.act, true);
  if (
    (resourceId !== undefined && resource.id !== resourceId) ||
    (headRevision !== undefined &&
      (resource.revision > headRevision || (act && act.resource_revision > headRevision)))
  )
    invalid('La seleccion no corresponde al recurso y cabeza consultados.');
  return { resource, act, target: resourceActivityTarget(value.target) };
}
export function resourceActivityCommand(raw) {
  object(raw, [
    'case_id',
    'resource_id',
    'association_id',
    'operation_id',
    'expected_resource_revision',
    'change',
  ]);
  for (const key of ['case_id', 'resource_id', 'association_id', 'operation_id'])
    canonical(raw[key], uuid);
  const change = raw.change;
  if (!['link', 'unlink'].includes(change?.action)) invalid();
  const keys = ['action', 'expected_revision'];
  object(change, [
    ...keys,
    ...(change.action === 'link' ? ['resource', 'act', 'target'] : ['reason']),
  ]);
  revision(raw.expected_resource_revision);
  let result;
  if (change.action === 'link') {
    if (change.expected_revision !== 0) invalid();
    result = {
      ...change,
      ...resourceActivitySelection(
        { resource: change.resource, act: change.act, target: change.target },
        raw.resource_id,
        raw.expected_resource_revision,
      ),
    };
  } else {
    revision(change.expected_revision, 4294967294);
    result = { ...change, reason: text(change.reason, 'Motivo') };
  }
  return { ...raw, change: result };
}
export const resourceActivityKinds = { hearing: 'Audiencia', deadline: 'Plazo' };
export const resourceActivityStatuses = { linked: 'Vinculada', unlinked: 'Desvinculada' };
