export function metadataApi(scoped, caseId) {
  return {
    uploadWithMetadata(file, name, values) {
      const form = new FormData();
      form.append('file', file);
      form.append('metadata', new Blob([JSON.stringify(values)], { type: 'application/json' }));
      return scoped('/with-metadata', {
        method: 'POST',
        body: form,
        headers: { 'X-Document-Name': name },
      });
    },
    metadata(id) {
      let active = true;
      const assertActive = () => {
        if (!active) throw new Error('El documento de esta solicitud ya no est\u00e1 abierto.');
      };
      async function request(suffix = '', options) {
        assertActive();
        const result = await scoped(`/${encodeURIComponent(id)}/metadata${suffix}`, options);
        assertActive();
        if (result.case_id !== caseId || result.id !== id)
          throw new Error('La clasificaci\u00f3n no corresponde al documento abierto.');
        return result;
      }
      return {
        dispose: () => {
          active = false;
        },
        get: () => request(),
        replace: (expected, values) =>
          request('', { method: 'PUT', data: { expected_metadata_revision: expected, ...values } }),
        history({ limit = 50, beforeRevision } = {}) {
          const query = new URLSearchParams({ limit });
          if (beforeRevision !== undefined) query.set('before_revision', beforeRevision);
          return request(`/history?${query}`);
        },
      };
    },
  };
}
