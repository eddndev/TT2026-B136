import {
  incidentUuid,
  incidentQuery,
  incidentPageValue,
  incidentRecordValue,
} from './document-integrity-incident-values.mjs';

export function integrityIncidentsApi(request) {
  const base = '/document-integrity-incidents';
  let active = true;
  function assertActive() {
    if (!active) throw new Error('La consulta de incidentes ya no est\u00e1 abierta.');
  }
  async function call(path) {
    assertActive();
    try {
      const value = await request(path);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      throw error;
    }
  }
  return {
    dispose() {
      active = false;
    },
    async list(input) {
      assertActive();
      const query = incidentQuery(input);
      return incidentPageValue(await call(`${base}?${new URLSearchParams(query)}`), query);
    },
    async get(id) {
      assertActive();
      incidentUuid(id);
      return incidentRecordValue(await call(`${base}/${id}`), id);
    },
  };
}
