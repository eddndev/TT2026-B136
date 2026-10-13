import { dashboardValue } from './dashboard-values.mjs';

export function dashboardApi(request) {
  let active = true;
  const assertActive = () => {
    if (!active) throw new Error('La consulta del tablero ya no est\u00e1 abierta.');
  };
  return {
    dispose() {
      active = false;
    },
    async get() {
      assertActive();
      try {
        const value = await request('/dashboard');
        assertActive();
        return dashboardValue(value);
      } catch (failure) {
        assertActive();
        throw failure;
      }
    },
  };
}
