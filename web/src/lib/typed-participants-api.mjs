export function typedParticipantsApi(request, caseId) {
  let active = true;
  const base = `/cases/${encodeURIComponent(caseId)}`;
  const key = (id) => encodeURIComponent(id);
  const query = (values) =>
    new URLSearchParams(
      Object.entries(values).filter(([, value]) => value !== undefined && value !== ''),
    ).toString();
  function validate(record, id, revision) {
    if (!record || record.case_id !== caseId || (id && record.id !== id))
      throw new Error('La respuesta no corresponde al expediente y recurso consultados.');
    if (revision !== undefined && record.revision !== revision)
      throw new Error('La respuesta no corresponde a la revisi\u00f3n solicitada.');
    return record;
  }
  const assertActive = () => {
    if (!active) throw new Error('El expediente de esta solicitud ya no est\u00e1 abierto.');
  };
  async function call(suffix, options) {
    assertActive();
    const result = await request(`${base}${suffix}`, options);
    assertActive();
    return result;
  }
  async function proposalCall(action, data) {
    const result = validate(
      await call(`/participants/proposals/${action}`, { method: 'POST', data }),
    );
    const expectedId =
      action === 'commit'
        ? data.prepared.proposal.participant_id
        : action === 'prepare'
          ? data.proposal.participant_id
          : data.participant?.id;
    const actualId = action === 'commit' ? result.id : result.proposal?.participant_id;
    if (expectedId && expectedId !== actualId)
      throw new Error('La propuesta no corresponde al participante seleccionado.');
    return result;
  }
  return {
    dispose: () => {
      active = false;
    },
    review: (data) => proposalCall('review', data),
    prepare: (data) => proposalCall('prepare', data),
    commit: (data) => proposalCall('commit', data),
    async participantRevision(id, revision) {
      return validate(await call(`/participants/${key(id)}/revisions/${revision}`), id, revision);
    },
    async credential(id, revision) {
      const result = validate(
        await call(`/participants/${key(id)}/revisions/${revision}/credential`),
      );
      if (
        result.reference?.participant_id !== id ||
        result.reference?.participant_revision !== revision
      )
        throw new Error('La credencial no corresponde a la revisi\u00f3n solicitada.');
      return result;
    },
    async subjects({ limit = 50, afterId, name, kind } = {}) {
      const result = await call(`/subjects?${query({ limit, after_id: afterId, name, kind })}`);
      result.subjects.forEach((row) => validate(row));
      return result;
    },
    async subject(id) {
      return validate(await call(`/subjects/${key(id)}`), id);
    },
    async subjectRevision(id, revision) {
      return validate(await call(`/subjects/${key(id)}/revisions/${revision}`), id, revision);
    },
    async subjectHistory(id, { limit = 50, beforeRevision } = {}) {
      const result = await call(
        `/subjects/${key(id)}/history?${query({ limit, before_revision: beforeRevision })}`,
      );
      result.revisions.forEach((row) => validate(row, id));
      return result;
    },
    async reviewSubject(id, expected, values) {
      return validate(
        await call(`/subjects/${key(id)}/review`, {
          method: 'POST',
          data: { expected_revision: expected, values },
        }),
        id,
      );
    },
    async replaceSubject(id, expected, values, review) {
      return validate(
        await call(`/subjects/${key(id)}`, {
          method: 'PUT',
          data: { expected_revision: expected, values, review },
        }),
        id,
      );
    },
  };
}
