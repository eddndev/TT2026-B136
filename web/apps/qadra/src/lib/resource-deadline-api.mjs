import {
  factUuid as uuid,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineBudget } from './deadline-page.mjs';
import {
  resourceDeadlineCommand,
  resourceDeadlineDraft,
  resourceDeadlineMatches,
} from './resource-deadline-values.mjs';
export function resourceDeadlinesApi(request, caseId, resourceId) {
  caseId = uuid(caseId);
  resourceId = uuid(resourceId);
  const base = `/cases/${caseId}/procedural-resources/${resourceId}/activities/deadlines`;
  let active = true;
  function assertActive() {
    if (!active) invalid('El contexto del recurso ya no esta abierto.');
  }
  function scope(command) {
    if (command.case_id !== caseId || command.resource_id !== resourceId)
      invalid('El comando pertenece a otro recurso o expediente.');
  }
  async function call(suffix, data) {
    assertActive();
    deadlineBudget(data);
    try {
      const value = await request(base + suffix, { method: 'POST', data });
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      throw error;
    }
  }
  return {
    dispose() {
      active = false;
    },
    async prepare(raw, principal) {
      assertActive();
      deadlineBudget(raw);
      const command = resourceDeadlineCommand(structuredClone(raw));
      scope(command);
      const actorId = uuid(principal?.id);
      if (!principal.email || !['owner', 'litigator'].includes(principal.role)) invalid();
      const value = resourceDeadlineDraft(await call('/prepare', command));
      if (
        !same(value.command, command) ||
        value.deadline.actor_id !== actorId ||
        value.deadline.author.email !== principal.email
      )
        invalid('La preparacion no corresponde al comando y la identidad enviados.');
      return value;
    },
    async submit(raw) {
      assertActive();
      deadlineBudget(raw);
      const draft = resourceDeadlineDraft(structuredClone(raw));
      scope(draft.command);
      const result = await call('/submit', {
        command: draft.command,
        expected_submission_digest: draft.submission_digest,
      });
      if (!resourceDeadlineMatches(result, draft))
        invalid('No se pudieron confirmar ambos recibos exactos. Consulta el resultado del envio.');
      return result;
    },
  };
}
