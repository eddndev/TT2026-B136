async function owned(read) {
  try {
    return await read();
  } catch (failure) {
    if ([403, 404].includes(failure.status)) failure.draftOwner = true;
    throw failure;
  }
}
export async function readFactOwner(api, scoped, caseId, parentId, base, admitted) {
  if (parentId) {
    const parents = api.caseResolutions(caseId);
    try {
      await owned(() => parents.get(parentId));
    } finally {
      parents.dispose();
    }
    if (!admitted()) return null;
  }
  return base ? owned(() => scoped.get(base.id)) : null;
}
const selected = (ref) =>
  typeof ref?.id === 'string' &&
  ref.id.length > 0 &&
  Number.isSafeInteger(ref.revision) &&
  ref.revision > 0;
export async function refreshFactReferences(api, caseId, values, admitted) {
  if (selected(values.resolution)) {
    const parents = api.caseResolutions(caseId);
    try {
      await parents.revision(values.resolution.id, values.resolution.revision);
    } finally {
      parents.dispose();
    }
    if (!admitted()) return false;
  }
  const people = [values.intended_recipient, values.actual_receiver]
    .filter((row) => row?.kind === 'known')
    .map((row) => row.value);
  if (values.representation?.kind === 'declared')
    people.push(values.representation.represented, values.representation.representative);
  const typed = api.caseTypedParticipants(caseId);
  try {
    for (const row of people.filter(
      (person) => person?.kind === 'participant' && selected(person),
    )) {
      await typed.participantRevision(row.id, row.revision);
      if (!admitted()) return false;
    }
  } finally {
    typed.dispose();
  }
  const provenances = [[values.provenance, ['provenance', 'support']]];
  if (values.representation?.kind === 'declared')
    provenances.push([
      values.representation.provenance,
      ['representation', 'provenance', 'support'],
    ]);
  for (const [provenance, path] of provenances) {
    const reference = provenance?.reference;
    if (
      provenance?.kind === 'hearing_result' &&
      reference?.result_id &&
      Number.isSafeInteger(reference.revision)
    ) {
      const results = api.caseHearingResults(caseId, reference.hearing_id);
      try {
        const row = await results.revision(reference.result_id, reference.revision);
        if (
          reference.agreement_id !== null &&
          !row.values.agreements.some((item) => item.id === reference.agreement_id)
        )
          throw new Error('El acuerdo ya no corresponde a la revision seleccionada.');
      } finally {
        results.dispose();
      }
      if (!admitted()) return false;
    }
    const support = provenance?.support;
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
        draftSupport: path,
      });
    } finally {
      version.dispose();
      documents.dispose();
    }
  }
  return admitted();
}
