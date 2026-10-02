import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { lawyerId } from './case-reports-fixtures.mjs';
import {
  setupReports,
  enterReports,
  fillFilters,
  reports,
  form,
  details,
} from './case-reports-helpers.mjs';
const colleague = '83000000-0000-4000-8000-000000000002';
const choice = { user_id: colleague, email: 'closed.colleague@example.test' };
const select = (page) => form(page).getByLabel('Litigante asignado', { exact: true });
const loadMore = (page) =>
  form(page).getByRole('button', { name: 'Cargar m\u00e1s litigantes', exact: true });

test('a closed-only shared colleague can filter reports without reusing the active dashboard workload', async ({
  page,
}) => {
  const state = await setupReports(page, 'litigator');
  state.dashboard.workload = [];
  state.dashboard.active_cases = 0;
  state.picker.litigators = [choice];
  await enterReports(page);
  await expect(select(page)).toContainText(choice.email);
  expect(state.dashboardCalls).toBe(1);
  expect(state.calls.filter((call) => call.path.endsWith('/litigators'))).toHaveLength(1);
  await fillFilters(page, { status: 'closed', assigned: colleague });
  await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
  await expect(details(page)).toContainText('En cola');
  expect(state.requests).toHaveLength(1);
  expect(state.requests[0].filters).toMatchObject({
    status: 'closed',
    assigned_litigator: colleague,
  });
});

test('report litigator pagination loads only on explicit action and retains the selected first page choice', async ({
  page,
}) => {
  const state = await setupReports(page, 'litigator');
  state.picker.has_more = true;
  state.picker.next_after_id = lawyerId;
  state.handle = async (route, url) => {
    if (!url.pathname.endsWith('/litigators') || !url.searchParams.has('after_id')) return false;
    expect(url.searchParams.get('limit')).toBe('20');
    expect(url.searchParams.get('after_id')).toBe(lawyerId);
    await route.fulfill({
      json: { ...state.picker, litigators: [choice], has_more: false, next_after_id: null },
    });
    return true;
  };
  await enterReports(page);
  await select(page).selectOption(lawyerId);
  await expect(select(page)).not.toContainText(choice.email);
  expect(state.calls.filter((call) => call.path.endsWith('/litigators'))).toHaveLength(1);
  await loadMore(page).click();
  await expect(select(page)).toContainText(choice.email);
  await expect(select(page)).toHaveValue(lawyerId);
  await expect(select(page)).toContainText('lawyer@example.test');
  await expect(loadMore(page)).toHaveCount(0);
  expect(state.calls.filter((call) => call.path.endsWith('/litigators'))).toHaveLength(2);
});

test('a report litigator page released after logout cannot restore private choices', async ({
  page,
}) => {
  const state = await setupReports(page, 'litigator');
  let capture;
  const captured = new Promise((resolve) => {
    capture = resolve;
  });
  state.handle = async (route, url) => {
    if (!url.pathname.endsWith('/litigators')) return false;
    capture(route);
    return true;
  };
  await login(page, false, false);
  await navigate(page, 'Informes');
  const route = await captured;
  await expect(reports(page)).toHaveAttribute('aria-busy', 'true');
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  const response = page.waitForResponse((value) => value.url() === route.request().url());
  await route.fulfill({ json: { ...state.picker, litigators: [choice] } });
  await (await response).finished();
  await expect(reports(page)).toHaveCount(0);
  await expect(page.getByText(choice.email, { exact: true })).toHaveCount(0);
  expect(state.requests).toHaveLength(0);
});
