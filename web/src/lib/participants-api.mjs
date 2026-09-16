export function participantsApi(request, caseId) {
  let active = true;
  const base = `/cases/${encodeURIComponent(caseId)}/participants`;
  const path = (id) => `/${encodeURIComponent(id)}`;
  const assertActive = () => {
    if (!active) throw new Error('El expediente de esta solicitud ya no est\u00e1 abierto.');
  };
  function validate(record, id) {
    if (record.case_id !== caseId || (id && record.id !== id))
      throw new Error('El participante no corresponde al expediente abierto.');
    return record;
  }
  async function call(suffix, options) {
    assertActive();
    const result = await request(`${base}${suffix}`, options);
    assertActive();
    return result;
  }
  const query = (values) =>
    new URLSearchParams(
      Object.entries(values).filter(([, value]) => value !== undefined && value !== ''),
    ).toString();
  return {
    dispose: () => {
      active = false;
    },
    async list({
      limit = 50,
      afterId,
      name,
      procedural_role,
      kind,
      profile,
      status = 'active',
    } = {}) {
      const page = await call(
        `?${query({ limit, after_id: afterId, name, procedural_role, kind, profile, status })}`,
      );
      page.participants.forEach((record) => validate(record));
      return page;
    },
    async get(id) {
      return validate(await call(path(id)), id);
    },
    async create(values) {
      return validate(await call('', { method: 'POST', data: values }));
    },
    async replace(id, expected, values) {
      return validate(
        await call(path(id), { method: 'PUT', data: { expected_revision: expected, ...values } }),
        id,
      );
    },
    async changeStatus(id, expected, status) {
      return validate(
        await call(`${path(id)}/directory-status`, {
          method: 'PUT',
          data: { expected_revision: expected, directory_status: status },
        }),
        id,
      );
    },
    async history(id, { limit = 50, beforeRevision } = {}) {
      const page = await call(
        `${path(id)}/history?${query({ limit, before_revision: beforeRevision })}`,
      );
      page.revisions.forEach((record) => validate(record, id));
      return page;
    },
  };
}
