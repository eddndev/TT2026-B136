import { combinedAgendaQuery } from './combined-agenda-query.mjs';
import { combinedAgendaPage } from './combined-agenda-values.mjs';

export function combinedAgendaApi(request) {
  let active = true;
  const assertActive = () => {
    if (!active) throw new Error('Esta consulta de agenda ya no esta abierta.');
  };
  return {
    dispose() {
      active = false;
    },
    async list(input) {
      assertActive();
      const { query, parameters } = combinedAgendaQuery(input);
      const value = await request(`/agenda?${parameters}`);
      assertActive();
      return combinedAgendaPage(value, query);
    },
  };
}
