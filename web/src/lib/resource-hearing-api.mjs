import { factInvalid as invalid } from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingOverview,
} from './resource-hearing-values.mjs';
import { resourceHearingCreation } from './resource-hearing-creation.mjs';

export function resourceHearingsApi(request, caseId, resourceId) {
  uuid(caseId);
  uuid(resourceId);
  const base = `/cases/${caseId}/procedural-resources/${resourceId}/activities/resource-hearings`;
  let active = true;
  const assertActive = () => {
    if (!active) invalid('La consulta de audiencia de recurso ya no esta abierta.');
  };
  return {
    dispose() {
      active = false;
    },
    async exact(overview) {
      assertActive();
      const selected = resourceHearingOverview(structuredClone(overview));
      if (selected.case_id !== caseId || selected.resource_id !== resourceId) invalid();
      try {
        const value = await request(`${base}/${selected.id}/revisions/${selected.revision}`);
        assertActive();
        return resourceHearingCreation(value, selected);
      } catch (failure) {
        assertActive();
        throw failure;
      }
    },
  };
}
