import { setup, login } from './helpers.mjs';

export const dashboardValue = (change = {}) => ({
  checked_at: '2026-09-27T10:15:30Z',
  scope: 'office',
  active_cases: 9,
  pending_contracts: 4,
  deadlines_overdue: 2,
  deadlines_due_48h: 3,
  deadlines_due_7d: 7,
  deadlines_unresolved: 1,
  workload: [
    {
      user_id: '11111111-1111-4111-8111-111111111111',
      email: 'lawyer@example.test',
      active_cases: 6,
    },
  ],
  ...change,
});
export const dashboard = (page) =>
  page.getByRole('region', { name: 'Indicadores operativos', exact: true });
export const metric = (page, name) => dashboard(page).locator(`[data-metric="${name}"]`);
export async function setupDashboard(page, role = 'owner') {
  await setup(page, role);
  const state = {
    calls: 0,
    handle: null,
    value: dashboardValue({ scope: role === 'owner' ? 'office' : 'assigned_cases' }),
  };
  await page.route('**/api/v1/dashboard', async (route) => {
    state.calls++;
    if (state.handle) return state.handle(route);
    return route.fulfill({ json: state.value });
  });
  return state;
}
export async function enterDashboard(page) {
  await login(page, false, false);
}
