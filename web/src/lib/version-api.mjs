export function versionApi(scoped) {
  const base = (id) => `/${encodeURIComponent(id)}/versions`;
  return {
    versions(id, { limit = 50, beforeVersion } = {}) {
      const query = new URLSearchParams({ limit });
      if (beforeVersion !== undefined) query.set('before_version', beforeVersion);
      return scoped(`${base(id)}?${query}`);
    },
    append: (id, expectedVersion, file, name) =>
      scoped(`${base(id)}?expected_version=${encodeURIComponent(expectedVersion)}`, {
        method: 'POST',
        body: file,
        headers: { 'X-Document-Name': name, 'Content-Type': 'application/octet-stream' },
      }),
    version(id, number) {
      let active = true;
      const assertActive = () => {
        if (!active) throw new Error('La versi\u00f3n de esta solicitud ya no est\u00e1 abierta.');
      };
      const request = async (suffix = '', options) => {
        assertActive();
        const result = await scoped(`${base(id)}/${encodeURIComponent(number)}${suffix}`, options);
        assertActive();
        return result;
      };
      return {
        dispose: () => {
          active = false;
        },
        detail: () => request(),
        seal: () => request('/seal', { method: 'POST' }),
        async verify() {
          const report = await request('/verify', { method: 'POST' });
          if (report.id !== id || report.version !== number) {
            throw new Error('El resultado no corresponde a la versi\u00f3n seleccionada.');
          }
          return report;
        },
        async evidence() {
          const result = await request('/evidence', { binary: true });
          if (result.documentId !== id || result.version !== String(number)) {
            throw new Error('La evidencia no corresponde a la versi\u00f3n seleccionada.');
          }
          return result;
        },
      };
    },
  };
}
