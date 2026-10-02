import { setup, login, navigate } from './helpers.mjs';
export const panel = (page) =>
  page.getByRole('region', { name: 'Actividad registrada', exact: true });
export const auditPage = (start = 0, count = 1, change = {}) => ({
  checked_at: '2026-10-02T10:20:30.123456789Z',
  snapshot_max_sequence: '9223372036854775807',
  events: Array.from({ length: count }, (_, index) => ({
    sequence: String(start + index),
    timestamp: `2026-10-01T12:00:00.${String(start + index).padStart(9, '0')}Z`,
    actor: 'system',
    action: 'document.read',
    resource: `record-${start + index}`,
  })),
  has_more: false,
  next_cursor: null,
  ...change,
});
export async function setupAudit(page, role = 'owner') {
  await setup(page, role);
  const state = { calls: [], value: auditPage(), handle: null };
  await page.route('**/api/v1/audit/events?**', async (route) => {
    state.calls.push(new URL(route.request().url()));
    if (state.handle) return state.handle(route);
    return route.fulfill({ json: state.value });
  });
  return state;
}
export async function enterAudit(page) {
  await login(page, false, false);
  await navigate(page, 'Auditor\u00eda');
  await panel(page).getByLabel('Desde (UTC, incluido)', { exact: true }).fill('2026-10-01');
  await panel(page).getByLabel('Hasta (UTC, excluido)', { exact: true }).fill('2026-10-02');
}
export async function consult(page) {
  await panel(page).getByRole('button', { name: 'Consultar actividad', exact: true }).click();
}
