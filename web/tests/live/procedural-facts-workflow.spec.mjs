import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  openFacts,
  openResolution,
  factDetail,
  factEditor,
  factList,
  fillNotification,
  prepare,
  submit,
} from './procedural-facts-helpers.mjs';

test('real facts preserve withdrawn sources and support versions through notification correction and withdrawal', async ({
  page,
}) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  await openFacts(page);
  await openResolution(page, accounts.withdrawnResolution);
  await expect(factDetail(page)).toContainText(accounts.resolution.values.summary);
  await expect(factDetail(page)).toContainText(accounts.hearingResult.values.summary);
  await expect(
    factDetail(page).getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true }),
  ).toHaveCount(0);
  await page.getByRole('button', { name: 'Ver notificaciones', exact: true }).click();
  await factList(page, 'notification')
    .getByRole('button', {
      name: `Consultar notificaci\u00f3n ${accounts.notification.id}`,
      exact: true,
    })
    .click();
  await expect(factDetail(page, 'notification')).toContainText(accounts.archived.display_name);
  await expect(factDetail(page, 'notification')).toContainText('fact-representation.docx');
  await factDetail(page, 'notification')
    .getByRole('button', { name: 'Corregir notificaci\u00f3n', exact: true })
    .click();
  const editor = factEditor(page, 'notification');
  await editor
    .getByLabel('Resumen de la notificaci\u00f3n', { exact: true })
    .fill('Precision posterior de la constancia');
  await editor.getByLabel('Motivo', { exact: true }).fill('Detalle comunicado');
  const draft = await prepare(page, 'notification');
  expect(draft.values.resolution).toEqual({
    id: accounts.resolution.id,
    revision: accounts.withdrawnResolution.revision,
  });
  expect(draft.sources.resolution.status).toBe('withdrawn');
  expect(draft.sources.participants[0].revision).toBe(accounts.archived.revision);
  expect(draft.sources.participants[0].directory_status).toBe('archived');
  expect(draft.sources.direct_supports).toHaveLength(2);
  expect(draft.sources.direct_supports.map((row) => row.version)).toEqual([1, 1]);
  expect(draft.sources.direct_supports.map((row) => row.format).sort()).toEqual(['docx', 'pdf']);
  const corrected = await submit(page, draft);
  expect(corrected.receipt.submission_digest).toBe(draft.submission_digest);
  await factDetail(page, 'notification')
    .getByRole('button', { name: 'Retirar notificaci\u00f3n', exact: true })
    .click();
  await editor.getByLabel('Motivo', { exact: true }).fill('Duplicado informado');
  const withdrawn = await submit(page, await prepare(page, 'notification'));
  expect(withdrawn.status).toBe('withdrawn');
  await factDetail(page, 'notification')
    .getByRole('button', { name: 'Ver historial de notificaci\u00f3n', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Consultar notificaci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await expect(factDetail(page, 'notification')).toContainText(
    accounts.notification.values.summary,
  );
  await page.getByRole('button', { name: 'Registrar notificaci\u00f3n', exact: true }).click();
  await fillNotification(page, 'Notificacion sin soportes directos');
  const withoutSupports = await prepare(page, 'notification');
  expect(withoutSupports.sources.direct_supports).toEqual([]);
  expect(withoutSupports.values.received_at).toBeNull();
  expect(withoutSupports.values.stated_effect).toBeNull();
  await submit(page, withoutSupports);
  expect(errors).toEqual([]);
});
