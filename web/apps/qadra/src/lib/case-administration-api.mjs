const query = (values) =>
  new URLSearchParams(
    Object.entries(values).filter(([, value]) => value !== undefined && value !== ''),
  ).toString();
export function caseAdministrationApi(request) {
  return {
    caseAdministrations: ({
      limit = 50,
      afterId,
      status = 'active',
      profile = 'all',
      title,
      nuc,
      judicial_case_number,
    } = {}) =>
      request(
        `/case-administrations?${query({ limit, after_id: afterId, status, profile, title, nuc, judicial_case_number })}`,
      ),
    createPenalCase: (values) => request('/penal-cases', { method: 'POST', data: values }),
    caseAdministration(id) {
      let active = true;
      const path = `/cases/${encodeURIComponent(id)}`;
      async function call(suffix, options) {
        if (!active) throw new Error('El expediente de esta solicitud ya no est\u00e1 abierto.');
        const result = await request(`${path}${suffix}`, options);
        if (!active) throw new Error('El expediente de esta solicitud ya no est\u00e1 abierto.');
        if (result.administration && (result.id !== id || result.administration.case_id !== id))
          throw new Error('La ficha no corresponde al expediente abierto.');
        if (result.revisions?.some((item) => item.case_id !== id))
          throw new Error('El historial no corresponde al expediente abierto.');
        return result;
      }
      return {
        dispose: () => {
          active = false;
        },
        get: () => call('/administration'),
        replace: (expected, values) =>
          call('/administration', {
            method: 'PUT',
            data: { expected_revision: expected, ...values },
          }),
        status: (expected, status) =>
          call('/administrative-status', {
            method: 'PUT',
            data: { expected_revision: expected, administrative_status: status },
          }),
        history: ({ limit = 50, beforeRevision } = {}) =>
          call(`/administration/history?${query({ limit, before_revision: beforeRevision })}`),
      };
    },
  };
}
