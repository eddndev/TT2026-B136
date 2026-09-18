import { factSame } from '../lib/procedural-fact-primitives.mjs';
function mismatch() {
  throw new Error('La referencia recibida no corresponde a la captura del plazo.');
}
export async function loadDeadlineReference(api, caseId, kind, captured) {
  let scoped, identity, revision;
  if (kind === 'profile') {
    scoped = api.deadlineProfiles(caseId);
    identity = captured.id;
    revision = captured.revision;
  } else if (kind === 'calendar') {
    scoped = api.judicialCalendars();
    identity = captured.id;
    revision = captured.revision;
  } else {
    const ref = captured.reference;
    revision = ref.revision;
    if (ref.family === 'resolution') {
      scoped = api.caseResolutions(caseId);
      identity = ref.id;
    } else if (ref.family === 'notification') {
      scoped = api.caseNotifications(caseId, ref.resolution.id);
      identity = ref.id;
    } else {
      scoped = api.caseHearingResults(caseId, ref.hearing_id);
      identity = ref.result_id;
    }
  }
  try {
    const row = await scoped.revision(identity, revision);
    if (row.receipt.submission_digest !== captured.submission_digest) mismatch();
    if (kind === 'profile') {
      if (
        row.definition_digest !== captured.definition_digest ||
        row.algorithm !== captured.algorithm ||
        row.definition.title !== captured.title ||
        !factSame(row.definition.scope, captured.scope)
      )
        mismatch();
    } else {
      if (row.values_digest !== captured.values_digest) mismatch();
      if (kind === 'source') {
        const ref = captured.reference;
        if (
          captured.sources_digest !== null &&
          row.receipt.sources_digest !== captured.sources_digest
        )
          mismatch();
        if (ref.family === 'notification' && !factSame(row.values.resolution, ref.resolution))
          mismatch();
        if (
          ref.family === 'hearing_result' &&
          ref.agreement_id !== null &&
          !row.values.agreements.some((item) => item.id === ref.agreement_id)
        )
          mismatch();
      }
    }
    return row;
  } finally {
    scoped.dispose();
  }
}
