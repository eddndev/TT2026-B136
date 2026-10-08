import { factSame, factInvalid } from '../lib/procedural-fact-primitives.mjs';

export async function refreshAdministrationSources(api, caseId, state, admitted) {
  const measures = api.caseMeasures(caseId),
    typed = api.caseTypedParticipants(caseId);
  try {
    const exact = await measures.exact(state.base.reference);
    if (!admitted()) return false;
    if (!factSame(exact, state.base))
      factInvalid('La medida exacta no coincide con el borrador conservado.');
    if (state.subject) {
      const exactSubject = await typed.subjectRevision(state.subject.id, state.subject.revision);
      if (!admitted()) return false;
      if (!factSame(exactSubject, state.subject))
        factInvalid('La identidad exacta no coincide con el borrador conservado.');
    }
    return admitted();
  } finally {
    measures.dispose();
    typed.dispose();
  }
}
