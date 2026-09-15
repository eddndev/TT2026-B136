import { test, expect } from '@playwright/test';
import {
  setupStages,
  openStages,
  chooseSupport,
  fillDate,
  initial,
  caseId,
  document,
  entry,
} from './stage-helpers.mjs';
import { navigate, login, otherCaseId, caseRecord } from './helpers.mjs';
import { administration, overview, profile } from './case-administration-helpers.mjs';
const time = { precision: 'date', date: '2026-09-01', offset: '-06:00' };
const intermediate = () =>
  entry({
    expected_revision: 1,
    target: 'intermediate',
    accusation_declared_at: time,
    accusation: { document_id: document.id, version: 1, digest: document.digest },
  });
test('trial separates emission and receipt and permits the same exact version in both roles', async ({
  page,
}, testInfo) => {
  const { state } = await setupStages(page, { current: intermediate() });
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Juicio' }).click();
  await fillDate(page, 'Fecha de emisi\u00f3n del auto');
  await expect(
    page
      .getByRole('group', { name: 'Fecha de recepci\u00f3n', exact: true })
      .getByLabel('Fecha', { exact: true }),
  ).toHaveValue('');
  await fillDate(page, 'Fecha de recepci\u00f3n');
  await page.getByLabel('Tribunal receptor', { exact: true }).fill('Tribunal declarado');
  await chooseSupport(page, 'Auto de apertura');
  await chooseSupport(page, 'Constancia de recepci\u00f3n');
  await page.getByRole('button', { name: 'Revisar registro' }).click();
  await expect(page.locator('.stage-confirmation')).toContainText('Tribunal declarado');
  await page.evaluate(() => {
    document.activeElement?.blur();
    scrollTo(0, 0);
  });
  await page.screenshot({
    path: testInfo.outputPath('stage-confirmation-desktop.png'),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: testInfo.outputPath('stage-confirmation-mobile.png'),
    fullPage: true,
  });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  const buttons = await page.locator('.stage-confirmation .action-row button').all();
  const first = await buttons[0].boundingBox(),
    second = await buttons[1].boundingBox();
  expect(first.x + first.width <= second.x + 1 || first.y + first.height <= second.y + 1).toBe(
    true,
  );

  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.locator('.stage-current')).toContainText('Juicio');
  expect(state.posts[0].opening_order).toEqual(state.posts[0].receipt_support);
  expect(state.posts[0].opening_order_issued_at).toEqual(time);
  expect(state.posts[0].received_at).toEqual(time);
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await expect(page.locator('.stage-history')).toHaveAttribute('aria-busy', 'false');
  await page.evaluate(() => {
    document.activeElement?.blur();
    scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath('stage-history-mobile.png'), fullPage: true });
});
test('selector serializes list, versions and exact read without auxiliary requests', async ({
  page,
}) => {
  const { requests } = await setupStages(page);
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  let release;
  await page.route(`**/api/v1/cases/${caseId}/documents?*`, async (route) => {
    await new Promise((resolve) => (release = resolve));
    await route.fulfill({ json: { documents: [document], has_more: false } });
  });
  await page.getByRole('button', { name: 'Elegir documento', exact: true }).click();
  await expect.poll(() => !!release).toBe(true);
  await expect(page.getByRole('button', { name: 'Ver historial de etapas' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Revisar registro' })).toBeDisabled();
  expect(requests.some((r) => /versions|metadata|verify|seal/.test(r.path))).toBe(false);
  release();
  const picker = page.getByRole('region', { name: 'Seleccionar soporte exacto' });
  await picker.getByRole('button', { name: /contrato.pdf/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1/ }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n' }).click();
  expect(requests.filter((r) => r.path.endsWith('/versions'))).toHaveLength(1);
  expect(requests.filter((r) => r.path.endsWith('/versions/1'))).toHaveLength(1);
  expect(requests.some((r) => /metadata|verify|seal/.test(r.path))).toBe(false);
});
test('historical support keeps version one while head and displayed filename are version two', async ({
  page,
}) => {
  const { state } = await setupStages(page);
  const newer = { ...document, version: 2, name: 'actual.pdf', digest: 'b'.repeat(64) };
  await page.route(`**/api/v1/cases/${caseId}/documents?*`, (route) =>
    route.fulfill({ json: { documents: [newer], has_more: false } }),
  );
  await page.route(`**/api/v1/cases/${caseId}/documents/*/versions?*`, (route) =>
    route.fulfill({
      json: {
        versions: [newer, document],
        has_more: false,
        first_available_version: 1,
        next_before_version: null,
      },
    }),
  );
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await fillDate(page, 'Fecha de la acusaci\u00f3n');
  await page.getByRole('button', { name: 'Elegir documento' }).click();
  await page.getByRole('button', { name: /actual.pdf/ }).click();
  await page.getByRole('button', { name: /Versi\u00f3n 1/ }).click();
  await page.getByRole('button', { name: 'Usar esta versi\u00f3n' }).click();
  await page.getByRole('button', { name: 'Revisar registro' }).click();
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.locator('.stage-current')).toContainText('Intermedia');
  expect(state.posts[0].accusation.version).toBe(1);
  expect(state.posts[0].accusation.digest).toBe(document.digest);
});
test('late stage head cannot resurrect a previous case after navigation', async ({ page }) => {
  await setupStages(page);
  const first = administration(caseRecord, 1, profile),
    second = administration(
      { ...caseRecord, id: otherCaseId, title: 'Otro expediente' },
      1,
      profile,
    );
  await page.route('**/api/v1/case-administrations?*', (route) =>
    route.fulfill({
      json: { cases: [overview(first), overview(second)], has_more: false, next_after_id: null },
    }),
  );
  await page.route(`**/api/v1/cases/${otherCaseId}/administration`, (route) =>
    route.fulfill({ json: second }),
  );
  let release;
  await page.route(`**/api/v1/cases/${caseId}/stage`, async (route) => {
    await new Promise((resolve) => (release = resolve));
    await route.fulfill({
      json: {
        case_id: caseId,
        current: { ...initial, recorded_by: { id: 'old', email: 'old-case@example.com' } },
      },
    });
  });
  await page.route(`**/api/v1/cases/${otherCaseId}/stage`, (route) =>
    route.fulfill({ json: { case_id: otherCaseId, current: null } }),
  );
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Etapas', exact: true }).click();
  await expect.poll(() => !!release).toBe(true);
  await page.getByRole('button', { name: 'Cambiar expediente' }).click();
  await page.getByRole('button', { name: /Otro expediente/ }).click();
  await page.getByRole('link', { name: 'Etapas', exact: true }).click();
  await expect(page.locator('.stage-current')).toContainText('Sin etapa registrada');
  release();
  await expect(page.getByText('old-case@example.com')).toHaveCount(0);
  await expect(page.locator('.stage-current')).toContainText('Sin etapa registrada');
});
