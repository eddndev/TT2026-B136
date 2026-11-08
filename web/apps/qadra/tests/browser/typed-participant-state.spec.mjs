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
