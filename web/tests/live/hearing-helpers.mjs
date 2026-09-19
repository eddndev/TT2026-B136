import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
export const accounts = fixture.hearings;
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de audiencia', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia', exact: true });
export async function openHearings(page) {
  await page.getByRole('link', { name: 'Audiencias', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Audiencias del expediente', exact: true }),
  ).toBeVisible();
  await expect(page.locator('.hearing-index')).toHaveAttribute('aria-busy', 'false');
}
export async function openHearing(page, id) {
  await page.getByRole('button', { name: `Consultar audiencia ${id}`, exact: true }).click();
  await expect(detail(page)).toBeVisible();
}
export async function fillHearing(page) {
  await editor(page).getByLabel('Fecha', { exact: true }).fill('2030-10-01');
  await editor(page).getByLabel('Hora', { exact: true }).fill('09:02:03');
  await editor(page).getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await editor(page)
    .getByLabel('Sede o conexi\u00f3n', { exact: true })
    .fill('Sede sintetica privada');
}
export async function prepare(page) {
  const response = page.waitForResponse(
    (r) => r.url().endsWith('/hearings/prepare') && r.request().method() === 'POST',
  );
  await editor(page).getByRole('button', { name: 'Revisar registro', exact: true }).click();
  const result = await response;
  expect(result.status()).toBe(200);
  return result.json();
}
export async function submit(page, prepared, expected = 201) {
  const command = prepared.command,
    action = command.change.action;
  const suffix =
    action === 'schedule'
      ? '/hearings'
      : `/hearings/${command.hearing_id}${action === 'cancel' ? '/cancellation' : ''}`;
  const response = page.waitForResponse(
    (r) =>
      r.url().endsWith(suffix) && r.request().method() === (action === 'replace' ? 'PUT' : 'POST'),
  );
  await editor(page).getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  const result = await response;
  expect(result.status()).toBe(expected);
  if (expected === 201) await expect(editor(page)).toHaveCount(0);
  return result.json();
}
export async function selectParticipant(page, name) {
  await editor(page).getByRole('button', { name: 'Elegir participante', exact: true }).click();
  await editor(page)
    .getByRole('button', { name: `Consultar ficha: ${name}`, exact: true })
    .click();
  await editor(page)
    .getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true })
    .click();
}
export async function ownerAction(index, action) {
  let token;
  async function call(method, path, data, expected = 200) {
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(data ? { 'Content-Type': 'application/json' } : {}),
      },
      body: data ? JSON.stringify(data) : undefined,
    });
    expect(response.status).toBe(expected);
    return response.status === 204 ? null : response.json();
  }
  const challenge = await call('POST', '/auth/login', {
    email: accounts.owner.email,
    password: accounts.owner.password,
  });
  token = (
    await call('POST', '/auth/mfa/recovery', {
      challenge_token: challenge.challenge_token,
      code: accounts.owner.recoveryCodes[index],
    })
  ).access_token;
  try {
    return await action(call);
  } finally {
    await call('POST', '/auth/logout', undefined, 204);
  }
}
export async function changeDirectory() {
  return ownerAction(4, async (call) => {
    const base = `/cases/${accounts.case.id}`,
      manual = `${base}/participants/${accounts.manual.id}`;
    await call('PUT', manual, {
      expected_revision: 1,
      display_name: 'Nombre actual archivado',
      procedural_role: 'Testigo',
      organization: null,
      legal_status: null,
      directory_status: 'active',
    });
    await call('PUT', `${manual}/directory-status`, {
      expected_revision: 2,
      directory_status: 'archived',
    });
    const subject = accounts.typed.subject,
      values = structuredClone(subject.values);
    values.name.value = 'Nombre actual de identidad';
    const path = `${base}/subjects/${subject.id}`;
    const review = await call('POST', `${path}/review`, { expected_revision: 1, values });
    await call('PUT', path, {
      expected_revision: 1,
      values,
      review: {
        directory_stamp: review.directory_stamp,
        selection_reason: 'Correccion sintetica de nombre',
        different: [],
      },
    });
  });
}
export async function advanceForCancellation() {
  return ownerAction(6, async (call) => {
    const support = accounts.typed.role_support;
    await call(
      'POST',
      `/cases/${accounts.case.id}/stage/transitions`,
      {
        expected_revision: 1,
        target: 'intermediate',
        accusation_declared_at: { precision: 'instant', at: '2026-09-01T10:00:00-06:00' },
        accusation: {
          document_id: support.document_id,
          version: support.version,
          digest: support.digest,
        },
        note: null,
      },
      201,
    );
  });
}
export async function queryAgenda(page) {
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption('custom');
  await page.getByLabel('Desde (incluido)', { exact: true }).fill('2030-10-01');
  await page.getByLabel('Hasta (excluido)', { exact: true }).fill('2030-10-03');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  await expect(page.locator('.hearing-agenda')).toHaveAttribute('aria-busy', 'false');
}
