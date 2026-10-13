import { test, expect } from '@playwright/test';
import {
  typedSetup,
  fillTyped,
  prepareTyped,
  typed,
  subjectId,
  support,
} from './typed-participant-helpers.mjs';
import { detail, openParticipant } from './participant-helpers.mjs';
test('retains an uncertain submission and reconciles exact origin without a second commit', async ({
  page,
}) => {
  const state = await typedSetup(page);
  let writes = 0;
  await page.route('**/participants/proposals/commit', async (route) => {
    writes++;
    state.records.set(typed.id, [typed]);
    await route.fulfill({ status: 503, json: { error: { code: 'service_busy' } } });
  });
  const dialog = await fillTyped(page);
  await prepareTyped(dialog);
  await dialog.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(
    dialog.getByRole('button', { name: 'Confirmar registro', exact: true }),
  ).toBeDisabled();
  await dialog.getByLabel('Nombre de la persona', { exact: true }).fill('Borrador posterior');
  await dialog
    .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  await expect(
    detail(page).getByRole('heading', { name: typed.display_name, exact: true }),
  ).toBeVisible();
  expect(writes).toBe(1);
});
test('changing values or certificate invalidates candidates, statement and signature', async ({
  page,
}) => {
  await typedSetup(page);
  const dialog = await fillTyped(page, 'control_judge');
  await dialog
    .getByLabel('Certificado p\u00fablico PEM o DER')
    .setInputFiles('tests/fixtures/participant-public-certificate.pem');
  await prepareTyped(dialog);
  await expect(
    dialog.getByRole('button', { name: 'Descargar declaraci\u00f3n binaria', exact: true }),
  ).toBeVisible();
  await dialog.getByLabel('\u00d3rgano jurisdiccional', { exact: true }).fill('Otro juzgado');
  await expect(
    dialog.getByRole('button', { name: 'Descargar declaraci\u00f3n binaria', exact: true }),
  ).toHaveCount(0);
  await expect(
    dialog.getByRole('heading', { name: 'Revisi\u00f3n de identidad', exact: true }),
  ).toHaveCount(0);
  await expect(
    dialog.getByRole('button', { name: 'Confirmar registro', exact: true }),
  ).toBeDisabled();
  await prepareTyped(dialog);
  await dialog.getByRole('button', { name: 'Quitar certificado', exact: true }).click();
  await expect(
    dialog.getByRole('button', { name: 'Descargar declaraci\u00f3n binaria', exact: true }),
  ).toHaveCount(0);
  await expect(
    dialog.getByRole('heading', { name: 'Revisi\u00f3n de identidad', exact: true }),
  ).toHaveCount(0);
});
test('does not merge a homonym and requires a supported explicit different-person decision', async ({
  page,
}) => {
  const state = await typedSetup(page);
  state.candidates = [
    {
      reference: { kind: 'subject', id: subjectId, revision: 1 },
      display_name: 'Persona tipificada',
      kind: 'natural_person',
      signals: ['name'],
    },
  ];
  const dialog = await fillTyped(page);
  await prepareTyped(dialog);
  await expect(dialog.getByRole('alert')).toContainText('cada candidato');
  const candidate = dialog.getByRole('region', {
    name: 'Candidato Persona tipificada',
    exact: true,
  });
  await candidate.getByRole('button', { name: 'Declarar persona distinta', exact: true }).click();
  await candidate
    .getByLabel('Motivo de candidato distinto', { exact: true })
    .fill('Homonimo distinto conforme documento');
  const field = candidate.getByRole('group', { name: 'Soporte de comparaci\u00f3n' });
  await field.getByRole('button', { name: 'Seleccionar documento', exact: true }).click();
  await field.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await field.getByRole('button', { name: /Versi\u00f3n 1 \/ contrato.pdf/ }).click();
  await field.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await field.getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }).fill('Pagina 1');
  await dialog.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await expect(
    dialog.getByText('Registro preparado sin firma personal', { exact: true }),
  ).toBeVisible();
  const call = state.calls.find((entry) => entry.path.endsWith('/prepare'));
  expect(call.body.review.different[0]).toEqual({
    candidate: state.candidates[0].reference,
    reason: 'Homonimo distinto conforme documento',
    support,
  });
});
test('typed editing preserves the subject and compares a fresh participant revision before new preparation', async ({
  page,
}) => {
  const state = await typedSetup(page, 'owner', [typed]);
  await openParticipant(page, typed.display_name);
  await detail(page).getByRole('button', { name: 'Editar participante', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Editar ficha tipificada', exact: true });
  await expect(
    dialog.getByRole('button', { name: 'Elegir identidad existente', exact: true }),
  ).toHaveCount(0);
  await dialog
    .getByLabel('Organizaci\u00f3n (opcional)', { exact: true })
    .fill('Borrador conservado');
  await prepareTyped(dialog);
  await page.route('**/participants/proposals/commit', async (route) => {
    state.records.get(typed.id).push({ ...typed, revision: 2, organization: 'Cambio concurrente' });
    await route.fulfill({
      status: 409,
      json: { error: { code: 'participant_revision_conflict' } },
    });
  });
  await dialog.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(
    dialog.getByRole('region', { name: 'Ficha actual consultada', exact: true }),
  ).toContainText('Cambio concurrente');
  await expect(dialog.getByLabel('Organizaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    'Borrador conservado',
  );
  await dialog
    .getByRole('button', { name: 'Usar esta base y conservar mi formulario', exact: true })
    .click();
  await dialog
    .getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true })
    .click();
  expect(
    state.calls.filter((call) => call.path.endsWith('/proposals/review')).at(-1).body.participant
      .expected_revision,
  ).toBe(2);
});
