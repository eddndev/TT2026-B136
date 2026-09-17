import { validateHearing } from './hearings-api.mjs';
import { hearingFailure } from './hearing-errors.mjs';
export function agendaApi(request) {
  let active = true;
  const assertActive = () => {
    if (!active) throw new Error('Esta consulta de agenda ya no est\u00e1 abierta.');
  };
  return {
    dispose: () => {
      active = false;
    },
    async list({ from, until, status = 'scheduled', limit = 20, after } = {}) {
      assertActive();
      const query = new URLSearchParams({ from, until, status, limit });
      if (after) {
        query.set('after_time', after.at);
        query.set('after_id', after.id);
      }
      let page;
      try {
        page = await request(`/hearings?${query}`);
      } catch (failure) {
        failure.message = hearingFailure(failure);
        throw failure;
      }
      assertActive();
      page.hearings.forEach((row) => validateHearing(row));
      return page;
    },
  };
}
