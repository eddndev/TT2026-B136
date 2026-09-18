import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  deadlineFixture,
  deadlineError,
} from './deadline-editor-helpers.mjs';
import { detail, id } from '../fixtures/deadline-unit.mjs';
import { login, navigate } from './helpers.mjs';
import { overview, otherAdministration } from './case-administration-helpers.mjs';
const panel = (page) => page.getByRole('region', { name: 'Detalle de plazo', exact: true });
const open = (page, row) =>
  page.getByRole('button', { name: `Consultar plazo ${row.id}`, exact: true }).click();
function gate() {
  let release;
  const promise = new Promise((resolve) => {
    release = resolve;
  });
  return { promise, release };
}

test('paralegal reads exact deadline history without any mutation or selector requests', async ({
  page,
}) => {
  const row = detail(deadlineFixture()),
    state = await setupDeadlines(page, { role: 'paralegal', deadlines: [row] });
  await openDeadlines(page);
  await open(page, row);
  await expect(panel(page)).toContainText(row.definition.title);
  for (const name of [
    'Registrar plazo',
    'Corregir plazo',
    'Declarar atenci\u00f3n',
    'Retirar plazo',
  ])
    await expect(page.getByRole('button', { name, exact: true })).toHaveCount(0);
  await panel(page).getByRole('button', { name: 'Ver historial de plazo', exact: true }).click();
  await page.getByRole('button', { name: 'Consultar plazo revisi\u00f3n 1', exact: true }).click();
  await expect(panel(page)).toContainText('Consultada exactamente');
  expect(state.calls.every((c) => c.method === 'GET')).toBe(true);
  expect(state.calls.some((c) => c.path.includes('/responsibles'))).toBe(false);
});
test('closed case keeps deadline history readable and creation disabled', async ({ page }) => {
  const row = detail(deadlineFixture()),
    state = await setupDeadlines(page, { closed: true, deadlines: [row] });
  await openDeadlines(page);
  await expect(page.getByRole('button', { name: 'Registrar plazo', exact: true })).toBeDisabled();
  await open(page, row);
  await expect(panel(page)).toContainText(row.definition.title);
  await expect(
    panel(page).getByRole('button', { name: 'Corregir plazo', exact: true }),
  ).toHaveCount(0);
  await panel(page).getByRole('button', { name: 'Ver historial de plazo', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Consultar plazo revisi\u00f3n 1', exact: true }),
  ).toBeVisible();
  expect(state.calls.every((c) => c.method === 'GET')).toBe(true);
});
test('client does not request deadlines through navigation or a protected hash', async ({
  page,
}) => {
  const state = await setupDeadlines(page, { role: 'client' });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Plazos', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'deadlines';
  });
  await expect(
    page.getByRole('heading', { name: 'Plazos del expediente', exact: true }),
  ).toHaveCount(0);
  expect(state.calls).toHaveLength(0);
});
for (const [status, code] of [
  [403, 'permission_denied'],
  [404, 'case_not_found'],
])
  test(`${code} removes previously displayed private deadline data`, async ({ page }) => {
    const row = detail(deadlineFixture()),
      state = await setupDeadlines(page, { deadlines: [row] });
    await openDeadlines(page);
    await open(page, row);
    await expect(panel(page)).toContainText(row.definition.title);
    state.handle = async (route) => {
      await deadlineError(route, code, status);
      return true;
    };
    await page.getByRole('button', { name: 'Actualizar plazos', exact: true }).click();
    await expect(
      page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
    ).toBeVisible();
    await expect(panel(page)).toHaveCount(0);
    await expect(page.getByText(row.definition.title, { exact: true })).toHaveCount(0);
  });
test('deadline_not_found is a local record error and does not revoke the case', async ({
  page,
}) => {
  const row = detail(deadlineFixture()),
    other = structuredClone(row);
  other.id = id(9);
  other.definition.title = 'Plazo no disponible';
  const state = await setupDeadlines(page, { deadlines: [row, other] });
  state.handle = async (route, call) => {
    if (!call.path.endsWith(`/${other.id}`)) return;
    await deadlineError(route, 'deadline_not_found', 404);
    return true;
  };
  await openDeadlines(page);
  await open(page, row);
  await expect(panel(page)).toBeVisible();
  await open(page, other);
  await expect(page.getByRole('alert')).toContainText(
    'El plazo o revisi\u00f3n exacta no est\u00e1 disponible',
  );
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Cambiar expediente', exact: true })).toBeVisible();
  await open(page, row);
  await expect(panel(page)).toContainText(row.definition.title);
});
for (const destination of ['case', 'session'])
  test(`late deadline detail is discarded after changing ${destination}`, async ({ page }) => {
    const row = detail(deadlineFixture()),
      state = await setupDeadlines(page, { deadlines: [row] }),
      held = gate();
    await page.route('**/api/v1/case-administrations?*', (route) =>
      route.fulfill({
        json: {
          cases: [overview(state.facts.results.scheduling.admin), overview(otherAdministration())],
          has_more: false,
          next_after_id: null,
        },
      }),
    );
    let started = false,
      finished = false;
    state.handle = async (route, call) => {
      if (!call.path.endsWith(`/${row.id}`)) return;
      started = true;
      await held.promise;
      await route.fulfill({ json: row });
      finished = true;
      return true;
    };
    await openDeadlines(page);
    await open(page, row);
    await expect.poll(() => started).toBe(true);
    if (destination === 'case') {
      await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
      await page.getByRole('button', { name: /Otro expediente/ }).click();
    } else await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
    held.release();
    await expect.poll(() => finished).toBe(true);
    await expect(panel(page)).toHaveCount(0);
    await expect(page.getByText(row.definition.title, { exact: true })).toHaveCount(0);
    expect(state.submissions).toHaveLength(0);
  });
