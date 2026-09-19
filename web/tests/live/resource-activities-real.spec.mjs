import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { openCase } from './procedural-resources-helpers.mjs';
import {
  accounts,
  panel,
  editor,
  detail,
  route,
  openActivities,
  chooseTarget,
  prepare,
  confirm,
  exactAssociation,
  resourceReference,
  linkCommand,
  accountAction,
  readyAlerts,
  alertSnapshot,
  unchangedTargets,
} from './resource-activities-real-helpers.mjs';

if (!accounts) throw new Error('The live fixture must provision independent resource activities');
for (const [name, width, role] of [
  ['desktop', 1440, 'owner'],
  ['mobile', 390, 'litigator'],
]) {
  test(`real exact resource associations preserve activities and alerts at ${width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(90000);
    const scenario = accounts[name],
      actor = accounts[role],
      errors = [],
      writes = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await loginAs(page, actor, 0);
    await accountAction(actor, 1, async (call) => {
      const beforeAlerts = await readyAlerts(call, scenario);
      await openActivities(page, scenario);
      page.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (path.startsWith('/api/v1/') && request.method() !== 'GET') writes.push(path);
      });
      await panel(page).getByRole('button', { name: 'Vincular actividad', exact: true }).click();
      await chooseTarget(page, scenario, 'hearing');
      await editor(page)
        .getByRole('button', { name: 'Elegir acto del recurso', exact: true })
        .click();
      await editor(page)
        .getByRole('combobox', { name: 'Acto del recurso', exact: true })
        .selectOption('2');
      await editor(page).getByRole('button', { name: 'Usar este acto', exact: true }).click();
      const draft = await prepare(page, scenario);
      expect(draft.recorded_by).toEqual({ id: actor.id, email: actor.email });
      expect(draft.selection.resource).toEqual(resourceReference(scenario.resource));
      expect(draft.selection.act).toEqual({
        id: scenario.resourceAct.act.id,
        revision: 1,
        resource_revision: 2,
        capture_digest: scenario.resourceAct.receipt.capture_digest,
      });
      expect(draft.sources.act).toEqual(scenario.resourceAct);
      expect(draft.sources.target.record).toEqual(scenario.hearingInitial);
      const linked = await confirm(page, scenario, draft);
      expect(linked.status).toBe('linked');
      await expect(
        detail(page).getByRole('region', { name: 'Captura vinculada', exact: true }),
      ).toContainText(scenario.hearingInitial.values.venue);
      await expect(
        detail(page).getByRole('region', { name: 'Actividad actual', exact: true }),
      ).toContainText(scenario.hearing.values.venue);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
      await page.screenshot({
        path: testInfo.outputPath(`resource-activity-real-${name}-linked.png`),
        fullPage: true,
      });
      await detail(page)
        .getByRole('button', { name: 'Desvincular actividad', exact: true })
        .click();
      await editor(page)
        .getByLabel('Motivo', { exact: true })
        .fill('Separacion organizativa sin cancelar la audiencia');
      const unlinked = await confirm(page, scenario, await prepare(page, scenario, true));
      expect(unlinked.status).toBe('unlinked');
      expect(unlinked.selection).toEqual(linked.selection);
      expect(unlinked.sources).toEqual(linked.sources);
      const historical = await exactAssociation(page, scenario, linked.id, 1);
      expect(historical.association).toEqual(linked);
      expect(historical.current_target.record).toEqual(scenario.hearing);
      await page.screenshot({
        path: testInfo.outputPath(`resource-activity-real-${name}-history.png`),
        fullPage: true,
      });
      await panel(page).getByRole('button', { name: 'Vincular actividad', exact: true }).click();
      await chooseTarget(page, scenario, 'deadline');
      const deadlineDraft = await prepare(page, scenario);
      expect(deadlineDraft.selection.target).toEqual({
        kind: 'deadline',
        id: scenario.deadlineInitial.id,
        revision: 1,
        capture_digest: scenario.deadlineInitial.receipt.capture_digest,
      });
      const deadlineLink = await confirm(page, scenario, deadlineDraft);
      const view = await call('GET', route(scenario) + `/${deadlineLink.id}`);
      expect(view.association.sources.target.record.revision).toBe(1);
      expect(view.current_target.record.revision).toBe(2);
      await unchangedTargets(call, scenario);
      expect(await alertSnapshot(call, scenario)).toEqual(beforeAlerts);
      expect(writes).toHaveLength(6);
      expect(writes.every((path) => path.startsWith('/api/v1' + route(scenario)))).toBe(true);
      expect(errors).toEqual([]);
    });
  });
}

test('real association access preserves read-only history and clears revoked membership', async ({
  page,
  browser,
}, testInfo) => {
  const scenario = accounts.policy;
  await page.goto('/');
  await loginAs(page, accounts.paralegal, 0);
  await openActivities(page, scenario);
  await panel(page)
    .getByRole('button', { name: `Consultar v\u00ednculo ${scenario.association.id}`, exact: true })
    .click();
  await expect(detail(page)).toBeVisible();
  await expect(
    panel(page).getByRole('button', { name: 'Vincular actividad', exact: true }),
  ).toHaveCount(0);
  await expect(
    detail(page).getByRole('button', { name: 'Desvincular actividad', exact: true }),
  ).toHaveCount(0);
  expect((await exactAssociation(page, scenario, scenario.association.id, 1)).association).toEqual(
    scenario.association,
  );
  await accountAction(accounts.paralegal, 1, async (call) => {
    const denied = await call('POST', route(scenario) + '/prepare', linkCommand(scenario), 403);
    expect(denied.error.code).toBe('permission_denied');
  });
  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const client = await context.newPage(),
      calls = [];
    client.on('request', (request) => calls.push(new URL(request.url()).pathname));
    await client.goto('/');
    await loginAs(client, accounts.client, 0);
    await openCase(client, scenario.case);
    await expect(client.getByRole('link', { name: 'Recursos', exact: true })).toHaveCount(0);
    expect(calls.filter((path) => path.includes('/procedural-resources'))).toEqual([]);
    await accountAction(accounts.client, 1, async (call) => {
      const denied = await call(
        'GET',
        route(scenario) + `/${scenario.association.id}`,
        undefined,
        403,
      );
      expect(denied.error.code).toBe('permission_denied');
    });
  } finally {
    await context.close();
  }
  await accountAction(accounts.owner, 2, async (call) => {
    await call(
      'DELETE',
      `/cases/${scenario.case.id}/members/${accounts.paralegal.id}`,
      undefined,
      204,
    );
  });
  await panel(page).getByRole('button', { name: 'Actualizar actividades', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  await expect(panel(page)).toHaveCount(0);
  await expect(detail(page)).toHaveCount(0);
  await expect(page.getByText(scenario.hearingInitial.values.venue, { exact: true })).toHaveCount(
    0,
  );
});
