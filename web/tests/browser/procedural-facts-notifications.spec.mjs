import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  factList,
  fillNotification,
  confirmFact,
} from './procedural-facts-helpers.mjs';
import {
  factRecord,
  factPrepared,
  factCommand,
  factKey,
  factResolutionSource,
} from '../fixtures/procedural-facts.mjs';

async function openChildren(page, row) {
  await openFacts(page);
  await factList(page)
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${row.id}`, exact: true })
    .click();
  await page.getByRole('button', { name: 'Ver notificaciones', exact: true }).click();
  await expect(factList(page, 'notification')).toBeVisible();
}

test('notification lifecycle retains its withdrawn exact parent without inheriting the parent support', async ({
  page,
}) => {
  const command = factCommand(),
    parent = factRecord(factPrepared(command));
  parent.values.provenance = {
    kind: 'external_reference',
    reference: 'Documento del antecedente',
    support: {
      document_id: '70000000-0000-4000-8000-000000000077',
      version: 1,
      digest: 'a'.repeat(64),
      locator: 'Pagina 1',
    },
  };
  parent.sources.direct_supports = [
    {
      document_id: parent.values.provenance.support.document_id,
      version: 1,
      digest: 'a'.repeat(64),
      name: 'padre.pdf',
      format: 'pdf',
      policy: 'pdf_docx_v1',
    },
  ];
  const retired = structuredClone(parent);
  retired.revision = 2;
  retired.status = 'withdrawn';
  retired.reason = 'Retiro administrativo';
  retired.receipt.action = 'withdraw';
  retired.receipt.expected_revision = 1;
  const state = await setupFacts(page, { facts: [parent, retired] });
  await openChildren(page, retired);
  await page.getByRole('button', { name: 'Registrar notificaci\u00f3n', exact: true }).click();
  const editor = factEditor(page, 'notification');
  for (const label of [
    'Car\u00e1cter de notificaci\u00f3n',
    'Medio de notificaci\u00f3n',
    'Contexto de notificaci\u00f3n',
    'Resultado declarado',
    'Precisi\u00f3n de pr\u00e1ctica',
    'Destinatario declarado',
    'Receptor material',
    'Representaci\u00f3n declarada',
  ])
    await expect(editor.getByRole('combobox', { name: label, exact: true })).toHaveValue('');
  await fillNotification(page);
  await confirmFact(page, 'notification');
  await expect(editor).toHaveCount(0);
  const first = state.submissions[0];
  expect(first.resolution_id).toBe(parent.id);
  expect(first.change.values.resolution).toEqual({ id: parent.id, revision: 2 });
  expect(first.change.values.received_at).toBeNull();
  expect(first.change.values.stated_effect).toBeNull();
  const record = state.records.get(factKey(first))[0];
  expect(record.sources.resolution.status).toBe('withdrawn');
  expect(record.sources.direct_supports).toEqual([]);
  await factDetail(page, 'notification')
    .getByRole('button', { name: 'Corregir notificaci\u00f3n', exact: true })
    .click();
  await editor
    .getByLabel('Resumen de la notificaci\u00f3n', { exact: true })
    .fill('Recepcion precisada sin efecto inferido');
  await editor.getByLabel('Motivo', { exact: true }).fill('Precision informada');
  await confirmFact(page, 'notification');
  await factDetail(page, 'notification')
    .getByRole('button', { name: 'Retirar notificaci\u00f3n', exact: true })
    .click();
  await editor.getByLabel('Motivo', { exact: true }).fill('Captura duplicada');
  await confirmFact(page, 'notification');
  await factDetail(page, 'notification')
    .getByRole('button', { name: 'Ver historial de notificaci\u00f3n', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Consultar notificaci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await expect(factDetail(page, 'notification')).toContainText(
    'Declaracion de notificacion capturada',
  );
  expect(state.submissions.map((row) => row.resolution_id)).toEqual([
    parent.id,
    parent.id,
    parent.id,
  ]);
  expect(state.submissions.map((row) => row.change.action)).toEqual([
    'record',
    'correct',
    'withdraw',
  ]);
  expect(
    state.calls
      .filter((call) => call.family === 'notification')
      .every((call) => call.parent === parent.id),
  ).toBe(true);
});

test('same notification UUID under another parent is never returned by child navigation', async ({
  page,
}) => {
  const parent = factRecord(),
    other = { ...structuredClone(parent), id: '90000000-0000-4000-8000-000000000009' };
  const command = factCommand('notification');
  const first = factRecord(factPrepared(command));
  first.values.summary = 'Notificacion del primer padre';
  first.sources.resolution = factResolutionSource(parent);
  const second = structuredClone(first);
  second.resolution_id = other.id;
  second.values.resolution.id = other.id;
  second.values.summary = 'Dato privado de otro padre';
  second.sources.resolution = factResolutionSource(other);
  const state = await setupFacts(page, { facts: [parent, other, first, second] });
  await openChildren(page, parent);
  await factList(page, 'notification')
    .getByRole('button', { name: `Consultar notificaci\u00f3n ${first.id}`, exact: true })
    .click();
  await expect(factDetail(page, 'notification')).toContainText(first.values.summary);
  await expect(page.getByText(second.values.summary, { exact: true })).toHaveCount(0);
  expect(
    state.calls
      .filter((call) => call.family === 'notification')
      .every((call) => call.parent === parent.id),
  ).toBe(true);
});
