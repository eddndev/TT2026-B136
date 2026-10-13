import { factObject as object } from './procedural-fact-primitives.mjs';
import { resourceHearingDigest as digest } from './resource-hearing-values.mjs';
import { resourceHearingCommand } from './resource-hearing-command.mjs';
import { resourceHearingActor, resourceHearingMaterial } from './resource-hearing-capture.mjs';

export function resourceHearingPrepared(row) {
  object(row, [
    'command',
    'resource',
    'act',
    'sources',
    'recorded_by',
    'observed_administration',
    'observed_resource_head',
    'submission_digest',
  ]);
  resourceHearingCommand(row.command);
  resourceHearingActor(row.recorded_by);
  digest(row.submission_digest);
  resourceHearingMaterial(
    row.command,
    row.resource,
    row.act,
    row.sources,
    row.observed_administration,
    row.observed_resource_head,
  );
  return row;
}
