import { factSame, factInvalid } from '../lib/procedural-fact-primitives.mjs';
import { precautionaryHearingRecordOverview } from '../lib/precautionary-hearing-record.mjs';

export async function refreshDecisionSources(api, caseId, state, admitted) {
  const typed = api.caseTypedParticipants(caseId),
    documents = api.caseDocuments(caseId),
    measures = api.caseMeasures(caseId),
    ordinary = api.caseHearings(caseId),
    precautionary = api.casePrecautionaryHearings(caseId);
  const same = (actual, expected) => {
    if (!factSame(actual, expected))
      factInvalid('La fuente exacta no coincide con el borrador conservado.');
  };
  try {
    if (state.support) {
      const row = state.support,
        client = documents.version(row.document_id ?? row.id, row.version);
      try {
        const exact = await client.detail();
        if (!admitted()) return false;
        if (
          exact.case_id !== caseId ||
          exact.version !== row.version ||
          exact.digest !== row.digest
        )
          factInvalid('El soporte exacto no coincide.');
      } finally {
        client.dispose();
      }
    }
    if (state.anchor?.kind === 'initial') {
      const row = state.anchor.record;
      const exact = await ordinary.revision(row.id, row.revision);
      if (!admitted()) return false;
      same(exact, row);
    } else if (state.anchor?.kind === 'precautionary') {
      const row = state.anchor.record;
      const exact = await precautionary.exact(precautionaryHearingRecordOverview(row));
      if (!admitted()) return false;
      same(exact, row);
    }
    if (state.fields.outcome !== 'changes') return admitted();
    for (const effect of state.effects) {
      const priors =
        effect.action === 'substitute'
          ? effect.predecessors
          : ['confirm', 'modify', 'revoke', 'cease'].includes(effect.action) && effect.previous
            ? [effect.previous]
            : [];
      for (const row of priors) {
        const exact = await measures.exact(row.reference);
        if (!admitted()) return false;
        same(exact, row);
      }
      const proposals =
        effect.action === 'substitute'
          ? effect.successors
          : ['impose', 'modify'].includes(effect.action)
            ? [effect.proposal]
            : [];
      for (const row of proposals) {
        if (row.subject) {
          const exact = await typed.subjectRevision(row.subject.id, row.subject.revision);
          if (!admitted()) return false;
          same(exact, row.subject);
        }
        if (row.supervisor) {
          const exact = await typed.participantRevision(row.supervisor.id, row.supervisor.revision);
          if (!admitted()) return false;
          same(exact, row.supervisor);
        }
      }
    }
    return admitted();
  } finally {
    for (const client of [typed, documents, measures, ordinary, precautionary]) client.dispose();
  }
}
