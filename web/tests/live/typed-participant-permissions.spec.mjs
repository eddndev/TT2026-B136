import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { openCase, navigate } from '../case-administration-workflow.mjs';
import { createPenal } from './case-administration-helpers.mjs';
import {
  accounts,
  directory,
  detail,
  participants,
  uploadIdentity,
  chooseSupport,
  prepare,
  submit,
} from './typed-participant-helpers.mjs';
async function withOwner(recoveryIndex, operation) {
  let token;
  async function call(method, path, data) {
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(data ? { 'Content-Type': 'application/json' } : {}),
      },
      body: data ? JSON.stringify(data) : undefined,
    });
    expect(response.ok).toBe(true);
    return response.status === 204 ? null : response.json();
  }
  const challenge = await call('POST', '/auth/login', {
    email: accounts.owner.email,
    password: accounts.owner.password,
  });
  const session = await call('POST', '/auth/mfa/recovery', {
    challenge_token: challenge.challenge_token,
    code: accounts.owner.recoveryCodes[recoveryIndex],
  });
  token = session.access_token;
  try {
    return await operation(call);
  } finally {
    await call('POST', '/auth/logout');
  }
}
test('institutional participants retain staff read policy, closed-case gates and revocation', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 2);
  const createdCase = await createPenal(
    page,
    'Politica de participantes tipificados',
    'TYPED-POLICY-001',
  );
  const record = { id: createdCase.id, title: createdCase.administration.title };
  await withOwner(3, async (call) => {
    for (const role of ['litigator', 'paralegal', 'client'])
      await call('PUT', `/cases/${record.id}/members/${accounts[role].id}`);
  });
  await participants(page);
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Agregar participante tipificado', exact: true });
  await dialog.getByLabel('Tipo de identidad', { exact: true }).selectOption('institutional_body');
  await dialog.getByLabel('Nombre del \u00f3rgano', { exact: true }).fill('Tribunal sintetico');
  await dialog
    .getByLabel('Motivo de Identificador institucional', { exact: true })
    .fill('Fixture sin registro oficial');
  await uploadIdentity(page, 'participant-body.pdf');
  await dialog.getByLabel('Tipo de participante', { exact: true }).selectOption('trial_court');
  await dialog.getByLabel('Distrito judicial', { exact: true }).fill('Distrito sintetico');
  await dialog
    .getByLabel('Composici\u00f3n del tribunal', { exact: true })
    .selectOption('collegiate');
  await chooseSupport(page, 'Soporte del rol', 'participant-body.pdf');
  await expect(
    dialog.getByLabel('Certificado p\u00fablico PEM o DER', { exact: true }),
  ).toHaveCount(0);
  await prepare(page, dialog, async (result) => {
    for (const candidate of result.candidates) {
      const row = dialog.getByRole('region', {
        name: `Candidato ${candidate.display_name}`,
        exact: true,
      });
      await row.getByRole('button', { name: 'Declarar persona distinta', exact: true }).click();
      await row
        .getByLabel('Motivo de candidato distinto', { exact: true })
        .fill('El organo sintetico es distinto de la persona representada');
      await chooseSupport(row, 'Soporte de comparaci\u00f3n', 'participant-body.pdf');
    }
  });
  const created = await submit(page, dialog);
  expect(created.subject.values.kind).toBe('institutional_body');
  expect(created.credential_origin).toBeNull();
  expect(created.profile.composition).toBe('collegiate');
  const contexts = [],
    readers = {};
  try {
    for (const role of ['litigator', 'paralegal', 'client']) {
      const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
      contexts.push(context);
      const reader = await context.newPage();
      readers[role] = reader;
      const paths = [];
      reader.on('pageerror', (error) => errors.push(error.message));
      reader.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (path.startsWith('/api/v1/')) paths.push(path);
      });
      await reader.goto('/');
      await loginAs(reader, accounts[role], 0);
      if (role === 'client') {
        await navigate(reader, 'Expedientes');
        await expect(
          reader.getByRole('button', { name: new RegExp(accounts.hiddenCase.title) }),
        ).toHaveCount(0);
        await reader.getByRole('button', { name: new RegExp(record.title) }).click();
        await expect(
          reader.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
        ).toBeVisible();
        await reader.evaluate(() => (location.hash = '#participants'));
        await expect(reader.getByRole('link', { name: 'Participantes', exact: true })).toHaveCount(
          0,
        );
        expect(paths.filter((path) => /\/(participants|subjects)(\/|$)/.test(path))).toEqual([]);
      } else {
        await openCase(reader, record);
        await participants(reader);
        await directory(reader)
          .getByRole('button', { name: 'Abrir Tribunal sintetico', exact: true })
          .click();
        await expect(detail(reader)).toContainText('Distrito sintetico');
        await detail(reader)
          .getByRole('button', { name: 'Consultar identidad actual', exact: true })
          .click();
        await expect(
          detail(reader).getByRole('region', { name: 'Identidad actual consultada', exact: true }),
        ).toContainText('Fixture sin registro oficial');
        if (role === 'paralegal') {
          await expect(
            reader.getByRole('button', { name: 'Agregar participante', exact: true }),
          ).toHaveCount(0);
          await expect(
            detail(reader).getByRole('button', { name: 'Editar identidad', exact: true }),
          ).toHaveCount(0);
        }
      }
    }
    await page.getByRole('link', { name: 'Resumen', exact: true }).click();
    await page.getByRole('button', { name: 'Cerrar administrativamente', exact: true }).click();
    await page
      .getByRole('button', { name: 'Confirmar cierre administrativo', exact: true })
      .click();
    await expect(
      page.getByRole('button', { name: 'Reactivar expediente', exact: true }),
    ).toBeVisible();
    await participants(page);
    await expect(
      page.getByRole('button', { name: 'Agregar participante', exact: true }),
    ).toBeDisabled();
    await directory(page)
      .getByRole('button', { name: 'Abrir Tribunal sintetico', exact: true })
      .click();
    await expect(
      detail(page).getByRole('button', { name: 'Editar participante', exact: true }),
    ).toBeDisabled();
    await detail(page)
      .getByRole('button', { name: 'Consultar identidad actual', exact: true })
      .click();
    await expect(
      detail(page).getByRole('button', { name: 'Editar identidad', exact: true }),
    ).toBeDisabled();
    await withOwner(4, (call) =>
      call('DELETE', `/cases/${record.id}/members/${accounts.litigator.id}`),
    );
    await directory(readers.litigator)
      .getByRole('button', { name: 'Actualizar', exact: true })
      .click();
    await expect(detail(readers.litigator)).toHaveCount(0);
    await expect(
      readers.litigator.getByText('Fixture sin registro oficial', { exact: false }),
    ).toHaveCount(0);
    await directory(readers.paralegal)
      .getByRole('button', { name: 'Actualizar', exact: true })
      .click();
    await expect(directory(readers.paralegal)).toContainText('Tribunal sintetico');
    expect(errors).toEqual([]);
  } finally {
    for (const context of contexts) await context.close();
  }
});
