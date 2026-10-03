export async function refreshResourceReferences(api, caseId, values, isAct, admitted) {
  if (!isAct && values.resolution?.id) {
    const resolutions = api.caseResolutions(caseId);
    try {
      await resolutions.revision(values.resolution.id, values.resolution.revision);
    } finally {
      resolutions.dispose();
    }
    if (!admitted()) return false;
  }
  if (!isAct) {
    const participants = api.caseTypedParticipants(caseId);
    try {
      for (const row of values.appellants)
        if (row.participant) {
          await participants.participantRevision(row.participant.id, row.participant.revision);
          if (!admitted()) return false;
        }
    } finally {
      participants.dispose();
    }
  }
  const references = isAct ? values.evidence : [values.resolution_evidence];
  for (let index = 0; index < references.length; index++) {
    const support = references[index];
    if (!support) continue;
    const documents = api.caseDocuments(caseId),
      version = documents.version(support.document_id, support.version);
    try {
      const row = await version.detail();
      if (!admitted()) return false;
      if (
        row.case_id !== caseId ||
        row.id !== support.document_id ||
        row.version !== support.version ||
        row.digest !== support.digest
      )
        throw new Error('El soporte no coincide con su version conservada.');
    } catch (failure) {
      throw Object.assign(new Error(failure.message), {
        code: failure.code,
        status: failure.status,
        resourceSupportIndex: index,
      });
    } finally {
      version.dispose();
      documents.dispose();
    }
  }
  return admitted();
}
