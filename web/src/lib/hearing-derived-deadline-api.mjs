import { factObject as object, factInvalid as invalid } from './procedural-fact-primitives.mjs';
import { deadlineBudget } from './deadline-page.mjs';
import { uuid, derivedCommand, derivedPrincipal } from './hearing-derived-deadline-command.mjs';
import { derivedReady, derivedRecord } from './hearing-derived-deadline-record.mjs';

export function hearingDerivedDeadlinesApi(request, caseId, hearingId) {
  caseId = uuid(caseId);
  hearingId = uuid(hearingId);
  const base = `/cases/${caseId}/hearings/${hearingId}/results/derived-deadline`;
  let active = true;
  function assertActive() {
    if (!active) invalid('El contexto de la audiencia ya no esta abierto.');
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
      const actor = derivedPrincipal(structuredClone(principal));
      const command = derivedCommand(structuredClone(raw), caseId, hearingId);
      const value = await call('/prepare', command);
      if (value?.state === 'ready') return derivedReady(value, command, actor.id);
      object(value, ['state', 'record']);
      if (value.state !== 'replay') invalid('La respuesta no identifica una revision o captura.');
      return { state: 'replay', record: derivedRecord(value.record, command, actor.id) };
    },
    async submit(raw, principal) {
      assertActive();
      const actor = derivedPrincipal(structuredClone(principal));
      const snapshot = structuredClone(raw);
      const command = derivedCommand(snapshot?.command, caseId, hearingId);
      const ready = derivedReady(snapshot, command, actor.id);
      const value = await call('/submit', {
        command,
        expected_review_digest: ready.review_digest,
      });
      return derivedRecord(value, command, actor.id, ready);
    },
  };
}
