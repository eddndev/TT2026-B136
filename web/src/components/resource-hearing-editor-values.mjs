import { hearingInstant } from '../lib/hearing-time.mjs';
import { resourceHearingCommand } from '../lib/resource-hearing-command.mjs';
import { factSame } from '../lib/procedural-fact-primitives.mjs';
import { resourceActivityFailure } from '../lib/resource-activity-errors.mjs';

export const resourceHearingReference = (row) => ({
  id: row.id,
  revision: row.revision,
  capture_digest: row.receipt.capture_digest,
});
export const supportKey = (row) => `${row.document_id}:${row.version}`;
export function compatibleHearingKind(resource) {
  if (resource.values.mode.kind !== 'known' || resource.values.mode.value !== 'written')
    return null;
  return (
    { appeal: 'appeal_arguments', revocation: 'written_revocation' }[resource.values.kind] || null
  );
}
export function initialHearingFields(resource) {
  return {
    kind: compatibleHearingKind(resource) || '',
    date: '',
    time: '',
    offset: '-06:00',
    modality: 'in_person',
    venue: '',
    note: '',
    statement: '',
    supportKey: resource.sources.supports[0] ? supportKey(resource.sources.supports[0]) : '',
  };
}
export function hearingSupports(resource, act) {
  const values = new Map();
  for (const row of [...resource.sources.supports, ...(act?.act.supports || [])]) {
    const key = supportKey(row),
      prior = values.get(key);
    if (prior && !factSame(prior, row))
      throw new Error(
        'Las capturas del soporte exacto no coinciden. Consulta de nuevo sus fuentes.',
      );
    values.set(key, row);
  }
  return [...values.values()];
}
export function hearingFormCommand(value) {
  const { resource, act, fields, base } = value;
  if (!compatibleHearingKind(resource) || compatibleHearingKind(base) !== fields.kind)
    throw new Error(
      'El tipo de recurso y su modalidad escrita deben corresponder a esta audiencia.',
    );
  const support = hearingSupports(resource, act).find(
    (row) => supportKey(row) === fields.supportKey,
  );
  if (!support)
    throw new Error('Selecciona un soporte admitido de las capturas del recurso o acto.');
  const { document_id, version, digest } = support;
  return resourceHearingCommand({
    case_id: value.caseId,
    resource_id: resource.id,
    operation_id: value.operationId,
    hearing_id: value.hearingId,
    association_id: value.associationId,
    expected_resource_revision: base.revision,
    resource: resourceHearingReference(resource),
    act: act
      ? {
          id: act.act.id,
          revision: act.act.revision,
          resource_revision: act.revision,
          capture_digest: act.receipt.capture_digest,
        }
      : null,
    values: {
      kind: fields.kind,
      scheduled_at: hearingInstant(fields),
      modality: fields.modality,
      venue: fields.venue.trim(),
      note: fields.note.trim() || null,
      participants: value.participants
        .map((row) => ({ participant_id: row.id, revision: row.revision }))
        .sort((a, b) => a.participant_id.localeCompare(b.participant_id)),
      scheduling_basis: {
        statement: fields.statement.trim(),
        support: { document_id, version, digest },
      },
    },
  });
}
export const resourceHearingFailure = (failure = {}) =>
  failure.status === 413
    ? 'La solicitud supera el limite de 64 KiB.'
    : failure.code === 'invalid_resource_hearing'
      ? 'Revisa la programacion, sus participantes y el soporte exacto del senalamiento.'
      : resourceActivityFailure(failure);

export async function refreshHearingSources(resources, typed, value, admitted) {
  const resource = await resources.revision(value.resource.id, value.resource.revision);
  if (!admitted()) return null;
  if (!factSame(resource, value.resource))
    throw new Error('La captura seleccionada del recurso cambio.');
  let act = null;
  if (value.act) {
    act = await resources.revision(value.resource.id, value.act.revision);
    if (!admitted()) return null;
    if (!factSame(act, value.act)) throw new Error('La captura seleccionada del acto cambio.');
  }
  const participants = [];
  for (const row of value.participants) {
    const exact = await typed.participantRevision(row.id, row.revision);
    if (!admitted()) return null;
    if (!factSame(exact, row)) throw new Error('La ficha seleccionada del participante cambio.');
    participants.push(exact);
  }
  return { resource, act, participants };
}
