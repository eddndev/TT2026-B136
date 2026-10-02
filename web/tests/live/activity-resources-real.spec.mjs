import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import { responseTo } from './procedural-resources-helpers.mjs';
import {
  accounts,
  accountAction,
  readyAlerts,
  route,
} from './resource-activities-real-helpers.mjs';
import { hearingOccurrenceMatches } from '../resource-activity-alert-origin.mjs';

if (!accounts?.policy?.association) throw new Error('The resource activities fixture is required');
const related = (page) => page.getByRole('region', { name: 'Recursos relacionados', exact: true });

test('real alert opens exact related resource and association without changing read state', async ({
  page,
}, testInfo) => {
  const scenario = accounts.policy,
    actor = accounts.owner,
    writes = [],
    errors = [];
  page.on('pageerror', (failure) => errors.push(failure.message));
  await page.goto('/');
  // Codes zero through four belong to existing resource and contextual scenarios.
  await loginAs(page, actor, 5);
  await accountAction(actor, 6, async (call) => {
    const alerts = await readyAlerts(call, scenario);
    const occurrence = alerts.find((row) => hearingOccurrenceMatches(row, scenario));
    const before = (await call('GET', `/alerts/${occurrence.id}`)).alert;
    expect(before.read_at).toBeNull();
    const origin = [scenario.hearingInitial, scenario.hearing].find(
      (row) => row.revision === before.origin.revision,
    );
    expect(before.origin.evidence_digest).toBe(origin.receipt.submission_digest);
    page.on('request', (request) => {
      const path = new URL(request.url()).pathname;
      if (path.startsWith('/api/v1/') && request.method() !== 'GET') writes.push(path);
    });
    await navigate(page, 'Alertas');
    await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
    await page.getByRole('combobox', { name: 'Lectura', exact: true }).selectOption('unread');
    await page.getByRole('combobox', { name: 'Estado de alerta', exact: true }).selectOption('all');
    await page.getByRole('button', { name: 'Consultar alertas', exact: true }).click();
    const card = page.locator(`[data-alert-id="${before.id}"]`);
    const exact = responseTo(
      page,
      `/api/v1/cases/${scenario.case.id}/hearings/${origin.id}/revisions/${origin.revision}`,
    );
    const inversePath = `/api/v1/cases/${scenario.case.id}/hearings/${origin.id}/resource-associations`;
    const reading = responseTo(page, inversePath);
    await card.getByRole('button', { name: 'Abrir audiencia', exact: true }).click();
    expect(await (await exact).json()).toEqual(origin);
    const response = await reading;
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toBe('no-store');
    const found = await response.json();
    expect(found.associations.map((row) => row.association)).toEqual([scenario.association]);
    expect(found.target).toEqual({ kind: 'hearing', id: origin.id });
    expect(found.associations[0].association.selection.target.revision).toBe(1);
    expect(found.associations[0].current_target.record).toEqual(scenario.hearing);
    await expect(related(page)).toContainText(
      `Revisi\u00f3n consultada de la actividad: ${origin.revision}`,
    );
    await expect(related(page)).toContainText('Revisi\u00f3n vinculada de la actividad: 1');
    for (const width of [1440, 390]) {
      await page.setViewportSize({ width, height: 1000 });
      await expect(related(page)).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
      await page.screenshot({
        path: testInfo.outputPath(`activity-resources-${width}.png`),
        fullPage: true,
      });
    }
    const captured = scenario.association.selection.resource;
    const resourceRead = responseTo(
      page,
      `/api/v1/cases/${scenario.case.id}/procedural-resources/${captured.id}/revisions/${captured.revision}`,
    );
    const associationRead = responseTo(
      page,
      '/api/v1' + route(scenario) + `/${scenario.association.id}/revisions/1`,
    );
    await related(page)
      .getByRole('button', {
        name: `Abrir recurso vinculado ${scenario.association.id}`,
        exact: true,
      })
      .click();
    expect(await (await resourceRead).json()).toEqual(scenario.resourceInitial);
    expect((await (await associationRead).json()).association).toEqual(scenario.association);
    await expect(
      page.getByRole('region', { name: 'Detalle de actividad vinculada', exact: true }),
    ).toContainText('Consulta historica exacta');
    await page.getByRole('button', { name: 'Volver a actividad', exact: true }).click();
    await expect(related(page)).toContainText(
      `Revisi\u00f3n consultada de la actividad: ${origin.revision}`,
    );
    await page.getByRole('button', { name: 'Volver a Alertas', exact: true }).click();
    await expect(page.getByRole('combobox', { name: 'Lectura', exact: true })).toHaveValue(
      'unread',
    );
    await expect(card).toContainText('Sin leer');
    expect((await call('GET', `/alerts/${before.id}`)).alert).toEqual(before);
    expect(writes).toEqual([]);
    expect(errors).toEqual([]);
  });
});
