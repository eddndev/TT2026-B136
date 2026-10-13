import {
  factObject as object,
  factInvalid as invalid,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingReference as reference,
  resourceHearingValues as values,
} from './resource-hearing-values.mjs';

export function resourceHearingBudget(value) {
  if (new TextEncoder().encode(JSON.stringify(value)).length > 65536)
    invalid('La solicitud supera el limite de 64 KiB.');
}

export function resourceHearingCommand(row) {
  object(row, [
    'case_id',
    'resource_id',
    'operation_id',
    'hearing_id',
    'association_id',
    'expected_resource_revision',
    'resource',
    'act',
    'values',
  ]);
  for (const key of ['case_id', 'resource_id', 'operation_id', 'hearing_id', 'association_id'])
    uuid(row[key]);
  revision(row.expected_resource_revision);
  reference(row.resource);
  if (row.act !== null) reference(row.act, true);
  if (
    row.resource.id !== row.resource_id ||
    row.resource.revision > row.expected_resource_revision ||
    (row.act !== null && row.act.resource_revision > row.expected_resource_revision)
  )
    invalid();
  values(row.values);
  return row;
}
