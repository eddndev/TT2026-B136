import { test, expect } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import {
  accounts,
  editor,
  detail,
  openResource,
  prepare,
  submit,
  exact,
  chooseSupport,
  accountAction,
} from './procedural-resources-helpers.mjs';

if (!accounts) throw new Error('The live fixture must provision independent procedural resources');
for (const [name, width, role] of [
  ['desktop', 1440, 'owner'],
  ['mobile', 390, 'litigator'],
]) {
  test(`real resource act correction preserves exact history at ${width}px`, async ({
    page,
  }, testInfo) => {
    const scenario = accounts[name],
      actor = accounts[role],
      errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await loginAs(page, actor, 0);
    expect(await openResource(page, scenario)).toEqual(scenario.resource);
    await expect(detail(page)).toContainText(scenario.resource.values.title);
    await detail(page).getByRole('button', { name: 'Registrar acto', exact: true }).click();
    await expect(
      editor(page).getByRole('combobox', { name: 'Tipo de acto', exact: true }),
    ).toHaveValue('');
    await editor(page)
      .getByRole('combobox', { name: 'Tipo de acto', exact: true })
      .selectOption('interposition');
    await editor(page)
      .getByRole('combobox', { name: 'Modalidad del acto', exact: true })
      .selectOption('oral');
    await editor(page)
      .getByRole('combobox', { name: 'Precisi\u00f3n de este acto', exact: true })
      .selectOption('date');
    await editor(page).getByLabel('Fecha de este acto', { exact: true }).fill('2026-09-19');
    await editor(page)
      .getByRole('combobox', { name: 'Autoridad del acto', exact: true })
      .selectOption('unknown');
    await editor(page)
      .getByLabel('Motivo: Autoridad del acto', { exact: true })
      .fill('No consta en esta captura');
    await editor(page)
      .getByLabel('Declaraci\u00f3n del acto', { exact: true })
      .fill(`Interposicion oral declarada ${name}`);
    await chooseSupport(page, scenario);
    const draft = await prepare(page, scenario);
    expect(draft.recorded_by).toEqual({ id: actor.id, email: actor.email });
    expect(draft.values).toEqual(scenario.resource.values);
    expect(draft.sources).toEqual(scenario.resource.sources);
    expect(draft.act.values.occurred_at).toEqual({
      precision: 'date',
      year: 2026,
      month: 9,
      day: 19,
      offset_seconds: null,
    });
    expect(draft.act.supports[0]).toMatchObject({
      document_id: scenario.support.id,
      version: 1,
      digest: scenario.support.digest,
    });
    const recorded = await submit(page, scenario, draft);
    expect(recorded.revision).toBe(2);
    expect(recorded.act.revision).toBe(1);
    expect(recorded.receipt.submission_digest).toBe(draft.submission_digest);
    expect(recorded.recorded_by).toEqual(draft.recorded_by);
    await detail(page).getByRole('button', { name: 'Corregir este acto', exact: true }).click();
    await editor(page)
      .getByLabel('Declaraci\u00f3n del acto', { exact: true })
      .fill(`Precision posterior del acto ${name}`);
    await editor(page).getByLabel('Motivo', { exact: true }).fill('Detalle documental comunicado');
    const revisedDraft = await prepare(page, scenario),
      corrected = await submit(page, scenario, revisedDraft);
    expect(corrected.revision).toBe(3);
    expect(corrected.act.revision).toBe(2);
    expect(corrected.act.id).toBe(recorded.act.id);
    expect(corrected.act.previous).toEqual({
      revision: 2,
      capture_digest: recorded.receipt.capture_digest,
    });
    expect(corrected.values).toEqual(scenario.resource.values);
    expect(corrected.sources).toEqual(scenario.resource.sources);
    expect(await exact(page, scenario, 2)).toEqual(recorded);
    await expect(detail(page)).toContainText(`Interposicion oral declarada ${name}`);
    await expect(detail(page)).not.toContainText(`Precision posterior del acto ${name}`);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({
      path: testInfo.outputPath(`resource-real-${name}-historical-act.png`),
      fullPage: true,
    });
    expect(await exact(page, scenario, 1)).toEqual(scenario.resource);
    await expect(
      detail(page).getByRole('heading', { name: 'Acto de esta revisi\u00f3n', exact: true }),
    ).toHaveCount(0);
    await detail(page)
      .getByRole('button', { name: 'Consultar recurso actual', exact: true })
      .click();
    await expect(detail(page)).toContainText(`Precision posterior del acto ${name}`);
    await expect(detail(page)).toContainText('Revisi\u00f3n del acto: 2');
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({
      path: testInfo.outputPath(`resource-real-${name}-current.png`),
      fullPage: true,
    });
    expect(errors).toEqual([]);
  });
}

test('real resource permissions retain staff history and clear it after membership revocation', async ({
  page,
  browser,
}, testInfo) => {
  const scenario = accounts.policy;
  await page.goto('/');
  await loginAs(page, accounts.paralegal, 0);
  expect(await openResource(page, scenario)).toEqual(scenario.resource);
  await expect(page.getByRole('button', { name: 'Registrar recurso', exact: true })).toHaveCount(0);
  await expect(
    detail(page).getByRole('button', { name: 'Registrar acto', exact: true }),
  ).toHaveCount(0);
  expect(await exact(page, scenario, 1)).toEqual(scenario.resource);
  await accountAction(accounts.paralegal, 1, async (call) => {
    const rejected = await call(
      'POST',
      `/cases/${scenario.case.id}/procedural-resources/prepare`,
      {
        operation_id: randomUUID(),
        resource_id: scenario.resource.id,
        change: {
          action: 'correct',
          expected_revision: 1,
          values: scenario.resource.values,
          reason: 'Unauthorized correction',
        },
      },
      403,
    );
    expect(rejected.error.code).toBe('permission_denied');
  });
  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const client = await context.newPage(),
      calls = [];
    client.on('request', (request) => calls.push(new URL(request.url()).pathname));
    await client.goto('/');
    await loginAs(client, accounts.client, 0);
    await navigate(client, 'Expedientes');
    await client.getByRole('button', { name: new RegExp(scenario.case.title) }).click();
    await expect(
      client.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
    ).toBeVisible();
    await expect(client.getByRole('link', { name: 'Recursos', exact: true })).toHaveCount(0);
    expect(
      calls.filter(
        (path) => path.startsWith('/api/v1/cases/') && path.includes('/procedural-resources'),
      ),
    ).toEqual([]);
    await accountAction(accounts.client, 1, async (call) => {
      const denied = await call(
        'GET',
        `/cases/${scenario.case.id}/procedural-resources/${scenario.resource.id}`,
        undefined,
        403,
      );
      expect(denied.error.code).toBe('permission_denied');
    });
  } finally {
    await context.close();
  }
  await accountAction(accounts.owner, 1, async (call) => {
    await call(
      'DELETE',
      `/cases/${scenario.case.id}/members/${accounts.paralegal.id}`,
      undefined,
      204,
    );
  });
  await page.getByRole('button', { name: 'Actualizar recursos', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  await expect(detail(page)).toHaveCount(0);
});
