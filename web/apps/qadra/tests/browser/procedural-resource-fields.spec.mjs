import { test, expect } from '@playwright/test';
import {
  setupProceduralResources,
  openResources,
  resourceEditor,
  resourceDetail,
} from './procedural-resources-helpers.mjs';
import { factRecord } from '../fixtures/procedural-facts.mjs';

async function unknown(editor, label) {
  await editor.getByRole('combobox', { name: label, exact: true }).selectOption('unknown');
  await editor.getByLabel(`Motivo: ${label}`, { exact: true }).fill('No consta en la fuente');
}
async function support(editor, label) {
  await editor.getByRole('button', { name: `Elegir soporte: ${label}`, exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1 \/ / }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await editor.getByLabel(`Localizador documental: ${label}`, { exact: true }).fill('Pagina 1');
}
async function fillResource(page, resolution) {
  const editor = resourceEditor(page);
  await editor
    .getByRole('combobox', { name: 'Tipo de recurso', exact: true })
    .selectOption('revocation');
  await editor
    .getByRole('combobox', { name: 'Modalidad del recurso', exact: true })
    .selectOption('written');
  await editor
    .getByLabel('Titulo organizativo', { exact: true })
    .fill('Revocacion escrita declarada');
  await editor
    .getByRole('button', { name: 'Elegir resoluci\u00f3n hist\u00f3rica', exact: true })
    .click();
  await editor
    .getByRole('button', {
      name: `Consultar revisiones de resoluci\u00f3n ${resolution.id}`,
      exact: true,
    })
    .click();
  await editor
    .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await editor
    .getByRole('button', { name: 'Vincular esta revisi\u00f3n de resoluci\u00f3n', exact: true })
    .click();
  await support(editor, 'la resoluci\u00f3n impugnada');
  await unknown(editor, 'Referencia de la resoluci\u00f3n');
  await unknown(editor, 'Autoridad emisora');
  await editor
    .getByRole('combobox', { name: 'Precisi\u00f3n de la resoluci\u00f3n', exact: true })
    .selectOption('unknown');
  await editor.getByLabel('Parte impugnada', { exact: true }).fill('Apartado segundo declarado');
  await editor.getByLabel('Motivos del recurso', { exact: true }).fill('Motivos manifestados');
  await editor.getByLabel('Nombre de recurrente 1', { exact: true }).fill('Persona sin ficha');
  await unknown(editor, 'Rol de recurrente 1');
}
async function confirm(page) {
  const editor = resourceEditor(page);
  await editor.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(editor).toHaveCount(0);
}

test('resource fields require explicit choices and keep an older exact resolution without declaring an act', async ({
  page,
}) => {
  const old = factRecord(),
    later = structuredClone(old);
  later.revision = 2;
  later.status = 'withdrawn';
  later.reason = 'Retiro declarado';
  later.receipt = { ...later.receipt, action: 'withdraw', expected_revision: 1 };
  const state = await setupProceduralResources(page, { facts: [old, later] });
  await openResources(page);
  await page.getByRole('button', { name: 'Registrar recurso', exact: true }).click();
  const editor = resourceEditor(page);
  await expect(editor.getByRole('combobox', { name: 'Tipo de recurso', exact: true })).toHaveValue(
    '',
  );
  await expect(
    editor.getByRole('combobox', { name: 'Modalidad del recurso', exact: true }),
  ).toHaveValue('');
  await expect(
    editor.getByRole('combobox', { name: 'Precisi\u00f3n de la resoluci\u00f3n', exact: true }),
  ).toHaveValue('');
  await fillResource(page, old);
  await confirm(page);
  const command = state.submissions.at(-1);
  expect(command.change.action).toBe('register');
  expect(command.change.values.resolution).toEqual({ id: old.id, revision: 1 });
  expect(command.change.values.mode).toEqual({ kind: 'known', value: 'written' });
  expect(command.change.values.resolution_at).toEqual({ precision: 'unknown' });
  expect(command.change.values.notification_at).toBeNull();
  expect(command.change.values.receiving_authority).toBeNull();
  expect(command.change.values.appellants[0].participant).toBeNull();
  await expect(resourceDetail(page)).toContainText('Persona sin ficha');
});

test('oral act records a declared date without inventing time and requires one or two exact supports', async ({
  page,
}) => {
  const old = factRecord(),
    state = await setupProceduralResources(page, { facts: [old] });
  await openResources(page);
  await page.getByRole('button', { name: 'Registrar recurso', exact: true }).click();
  await fillResource(page, old);
  await confirm(page);
  await page.getByRole('button', { name: 'Registrar acto', exact: true }).click();
  const editor = resourceEditor(page);
  await expect(editor.getByRole('combobox', { name: 'Tipo de acto', exact: true })).toHaveValue('');
  await editor
    .getByRole('combobox', { name: 'Tipo de acto', exact: true })
    .selectOption('interposition');
  await editor
    .getByRole('combobox', { name: 'Modalidad del acto', exact: true })
    .selectOption('oral');
  await editor
    .getByRole('combobox', { name: 'Precisi\u00f3n de este acto', exact: true })
    .selectOption('date');
  await editor.getByLabel('Fecha de este acto', { exact: true }).fill('2026-09-19');
  await expect(editor.getByLabel('Hora de este acto', { exact: true })).toHaveCount(0);
  await unknown(editor, 'Autoridad del acto');
  await editor
    .getByLabel('Declaraci\u00f3n del acto', { exact: true })
    .fill('Interposicion oral segun constancia');
  await support(editor, 'acto 1');
  await editor.getByRole('button', { name: 'Agregar soporte del acto', exact: true }).click();
  await expect(
    editor.getByRole('button', { name: 'Agregar soporte del acto', exact: true }),
  ).toBeDisabled();
  await editor.getByRole('button', { name: 'Quitar soporte adicional 2', exact: true }).click();
  await confirm(page);
  const command = state.submissions.at(-1);
  expect(command.change.action).toBe('record_act');
  expect(command.change.values.mode).toEqual({ kind: 'known', value: 'oral' });
  expect(command.change.values.occurred_at).toEqual({
    precision: 'date',
    year: 2026,
    month: 9,
    day: 19,
    offset_seconds: null,
  });
  expect(command.change.values.evidence).toHaveLength(1);
});
