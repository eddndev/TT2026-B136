import { expect } from '@playwright/test';
import { setup, login, navigate, caseId, id, document } from './helpers.mjs';
import { incidentRecord, incidentPage } from '../fixtures/document-integrity-incidents.mjs';

export const inbox = (page) =>
  page.getByRole('region', { name: 'Incidentes de integridad', exact: true });
export const incidentCard = (page, row) => inbox(page).locator(`[data-incident-id="${row.id}"]`);
export async function setupIncidents(page, role = 'owner') {
  const baseCalls = await setup(page, role);
  const rows = [incidentRecord(1), incidentRecord(2, 'authentication_failed')];
  rows[0].detected_at = '2026-09-20T10:00:00Z';
  rows[0].recorded_at = rows[0].detected_at;
  const state = { rows, calls: [], baseCalls, handle: null };
  await page.route(/\/api\/v1\/document-integrity-incidents(?:[/?]|$)/, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const call = {
      path: url.pathname,
      method: request.method(),
      limit: url.searchParams.get('limit'),
      after: url.searchParams.get('after_id'),
    };
    state.calls.push(call);
    if (state.handle && (await state.handle(route, call))) return;
    if (role !== 'owner')
      return route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } });
    const suffix = url.pathname.split('/document-integrity-incidents')[1];
    if (suffix) {
      const row = rows.find((value) => `/${value.id}` === suffix);
      return route.fulfill(row ? { json: row } : { status: 404 });
    }
    const remaining = rows.filter((row) => !call.after || row.id > call.after);
    return route.fulfill({ json: incidentPage(remaining.slice(0, 1), remaining.length > 1) });
  });
  await page.route(`**/cases/${caseId}/documents/**`, (route) => {
    const request = route.request(),
      url = new URL(request.url());
    state.baseCalls.push({ path: url.pathname, method: request.method() });
    const suffix = url.pathname.split(`/documents/${id}`)[1];
    if (suffix?.startsWith('/metadata')) return route.fallback();
    if (suffix === '')
      return route.fulfill({ json: { ...document, version: 2, name: 'actual.pdf' } });
    if (suffix === '/versions')
      return route.fulfill({
        json: {
          versions: [{ ...document, version: 2, name: 'actual.pdf' }, document],
          has_more: false,
          next_before_version: null,
          first_available_version: 1,
        },
      });
    if (suffix === '/versions/1') return route.fulfill({ json: document });
    return route.fallback();
  });
  return state;
}
export async function enterIncidents(page) {
  await login(page, false, false);
  await navigate(page, 'Incidentes de integridad');
  await expect(inbox(page)).toBeVisible();
}
