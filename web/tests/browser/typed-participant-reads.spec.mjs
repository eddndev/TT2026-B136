import { test, expect } from '@playwright/test';
import { typedSetup, typed, subject, subjectId } from './typed-participant-helpers.mjs';
import { detail, openParticipant } from './participant-helpers.mjs';
test('edits current identity separately while preserving the identity bound to the participant', async ({
  page,
}) => {
  const state = await typedSetup(page, 'owner', [typed]);
  await page.route('**/subjects/*', async (route) => {
    if (route.request().method() !== 'PUT') return route.fallback();
    const body = route.request().postDataJSON();
    const record = { ...subject, revision: 2, values: body.values };
    state.subjects.get(subjectId).push(record);
    state.subjectWrite = body;
    await route.fulfill({ json: record });
  });
  await page.route('**/subjects/*/review', async (route) => {
    const body = route.request().postDataJSON();
    await route.fulfill({
      json: {
        case_id: subject.case_id,
        id: subject.id,
        ...body,
        directory_stamp: 'c'.repeat(64),
        candidates: [],
      },
    });
  });
  await openParticipant(page, typed.display_name);
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await detail(page).getByRole('button', { name: 'Editar identidad', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Editar identidad del expediente', exact: true });
  await dialog.getByLabel('Nombre de la persona', { exact: true }).fill('Nombre actualizado');
  await dialog
    .getByRole('button', { name: 'Revisar coincidencias de identidad', exact: true })
    .click();
  await dialog
    .getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true })
    .fill('Documento de identidad actualizado');
  await dialog
    .getByRole('button', { name: 'Guardar revisi\u00f3n de identidad', exact: true })
    .click();
  await expect(dialog).not.toBeVisible();
  expect(state.subjectWrite.expected_revision).toBe(1);
  await expect(
    detail(page).getByRole('region', { name: 'Identidad actual consultada' }),
  ).toContainText('Nombre actualizado');
  await expect(detail(page)).toContainText('Identidad vinculada: revisi\u00f3n 1');
  expect(state.records.get(typed.id)).toHaveLength(1);
});
test('reads credential origin explicitly and exports only the public evidence and exact bytes', async ({
  page,
}) => {
  const origin = {
    participant_id: typed.id,
    participant_revision: 1,
    statement_digest: 'e'.repeat(64),
  };
  const state = await typedSetup(page, 'paralegal', [
    { ...typed, revision: 2, credential_origin: origin },
  ]);
  const evidence = {
    case_id: typed.case_id,
    reference: origin,
    declaration_base64: Buffer.alloc(218).toString('base64'),
    statement_digest: origin.statement_digest,
    certificate_der_base64: 'AA==',
    certificate_fingerprint: 'f'.repeat(64),
    certificate_summary: { subject: 'Firmante', issuer: 'CA interna', serial_hex: '01' },
    signature_base64: Buffer.alloc(384).toString('base64'),
    checked_at_unix: 1700000000,
    policy: 'internal_demo_v1',
    trust: { revision: 1, root_fingerprint: 'a'.repeat(64) },
  };
  await page.route('**/participants/*/revisions/*/credential', async (route) => {
    state.credentialPath = new URL(route.request().url()).pathname;
    await route.fulfill({ json: evidence });
  });
  await openParticipant(page, typed.display_name);
  expect(state.credentialPath).toBeUndefined();
  await detail(page)
    .getByRole('button', { name: 'Consultar evidencia de firma personal', exact: true })
    .click();
  await expect(detail(page)).toContainText('Firma comprobada con CA interna');
  expect(state.credentialPath).toContain('/revisions/1/credential');
  const pending = page.waitForEvent('download');
  await detail(page)
    .getByRole('button', { name: 'Descargar evidencia p\u00fablica', exact: true })
    .click();
  const stream = await (await pending).createReadStream(),
    chunks = [];
  for await (const chunk of stream) chunks.push(chunk);
  expect(JSON.parse(Buffer.concat(chunks).toString())).toEqual(evidence);
});

test('subject editing explicitly reads candidate details for comparison without changing the target identity', async ({
  page,
}) => {
  const state = await typedSetup(page, 'owner', [typed]);
  await page.route('**/subjects/*/review', async (route) => {
    const body = route.request().postDataJSON();
    await route.fulfill({
      json: {
        case_id: subject.case_id,
        id: subject.id,
        ...body,
        directory_stamp: 'c'.repeat(64),
        candidates: [
          {
            reference: { kind: 'subject', id: subjectId, revision: 1 },
            display_name: 'Otro candidato',
            kind: 'natural_person',
            signals: ['name'],
          },
        ],
      },
    });
  });
  await openParticipant(page, typed.display_name);
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await detail(page).getByRole('button', { name: 'Editar identidad', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Editar identidad del expediente', exact: true });
  await dialog
    .getByRole('button', { name: 'Revisar coincidencias de identidad', exact: true })
    .click();
  await dialog.getByRole('button', { name: 'Consultar candidato', exact: true }).click();
  await expect(
    dialog.getByRole('region', { name: 'Datos del candidato consultado', exact: true }),
  ).toContainText('Persona tipificada');
  expect(state.calls.some((call) => call.path.endsWith(`/subjects/${subjectId}/revisions/1`))).toBe(
    true,
  );
  await expect(dialog.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    'Persona tipificada',
  );
});
