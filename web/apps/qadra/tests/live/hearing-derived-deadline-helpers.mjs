import { expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { openResults, panel } from './hearing-result-helpers.mjs';

export const scenario = fixture.hearingDerivedDeadlines;
if (!scenario) throw new Error('Provision compound hearing and deadline fixtures before this spec');
export const scope = `/api/v1/cases/${scenario.case.id}`;
export const endpoint = `${scope}/hearings/${scenario.hearing.id}/results/derived-deadline`;
export const editor = (page) =>
  page.getByRole('region', { name: 'Resultado y plazo configurado', exact: true });
export const responseTo = (page, path, method = 'POST') =>
  page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === path && response.request().method() === method,
  );

export async function authenticate(page, index) {
  await page.goto('/');
  const pending = responseTo(page, '/api/v1/auth/mfa/recovery');
  await loginAs(page, scenario.owner, index);
  const token = (await (await pending).json()).access_token;
  return async (path) => {
    const response = await page.request.get(path, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(response.status(), `GET ${path}`).toBe(200);
    return response.json();
  };
}

export async function openEditor(page) {
  await openResults(page, scenario.case, scenario.hearing);
  await panel(page)
    .getByRole('button', { name: 'Registrar resultado y plazo', exact: true })
    .click();
  await expect(editor(page)).toBeVisible();
}

export async function fillEditor(page, title, timed) {
  const form = editor(page);
  await form.getByRole('combobox', { name: 'Ocurrencia', exact: true }).selectOption('occurred');
  await form
    .getByRole('combobox', { name: 'Alcance declarado', exact: true })
    .selectOption('partial');
  if (timed) {
    await form
      .getByRole('combobox', { name: 'Precisi\u00f3n', exact: true })
      .selectOption('instant');
    await form.getByLabel('Hora', { exact: true }).fill(scenario.instant.time);
  }
  await form.getByLabel('Fecha', { exact: true }).fill(scenario.instant.date);
  await form.getByLabel('Desfase UTC', { exact: true }).fill(scenario.instant.offset);
  await form
    .getByLabel('Relato del operador', { exact: true })
    .fill(`Resultado sintetico: ${title}`);
  await form
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('operator_note');
  await form.getByLabel('T\u00edtulo del plazo', { exact: true }).fill(title);
  await form.getByRole('button', { name: 'Elegir perfil exacto', exact: true }).click();
  await form
    .getByRole('button', {
      name: `Revisiones de ${scenario.profile.definition.title}`,
      exact: true,
    })
    .click();
  await form
    .getByRole('button', {
      name: 'Consultar perfil revisi\u00f3n 1 (publicado)',
      exact: true,
    })
    .click();
  await form.getByRole('button', { name: 'Usar este perfil exacto', exact: true }).click();
  await form
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('fixed');
  await form
    .getByRole('combobox', { name: 'Cuando cambie la fuente', exact: true })
    .selectOption('fixed');
  await form.getByRole('button', { name: 'Elegir responsable', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Elegir responsable', exact: true });
  const responsible = picker.getByRole('button', {
    name: `Elegir responsable ${scenario.owner.email}`,
    exact: true,
  });
  await expect(picker).toHaveAttribute('aria-busy', 'false');
  for (let nextPage = 0; (await responsible.count()) === 0 && nextPage < 10; nextPage++) {
    const next = picker.getByRole('button', { name: 'Siguientes responsables', exact: true });
    if (!(await next.count())) throw new Error('The responsible owner is not available');
    const loaded = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === `${scope}/deadlines/responsibles` && response.ok(),
    );
    await next.click();
    await loaded;
    await expect(picker).toHaveAttribute('aria-busy', 'false');
  }
  await responsible.click();
  await form
    .getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })
    .fill('Supuesto sintetico para verificar el recorrido, sin efecto juridico');
  await form.getByLabel('Localizador de aplicabilidad', { exact: true }).fill('Ejemplo declarado');
  await form
    .getByRole('combobox', { name: 'El ambito del perfil aplica', exact: true })
    .selectOption('yes');
  await form
    .getByRole('combobox', { name: 'Existe una incidencia sin resolver', exact: true })
    .selectOption('no');
  await form
    .getByRole('combobox', { name: 'Se cumple la condicion 1', exact: true })
    .selectOption('yes');
  await form
    .getByLabel('Localizador de condici\u00f3n 1', { exact: true })
    .fill('Ejemplo sintetico');
}

export async function prepare(page) {
  const pending = responseTo(page, `${endpoint}/prepare`);
  await editor(page)
    .getByRole('button', { name: 'Preparar resultado y plazo', exact: true })
    .click();
  const response = await pending;
  expect(response.status()).toBe(200);
  const ready = await response.json();
  expect(ready.state).toBe('ready');
  await expect(
    editor(page).getByRole('heading', {
      name: 'Revisa el resultado y el plazo',
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    editor(page).getByRole('button', {
      name: 'Confirmar resultado y plazo',
      exact: true,
    }),
  ).toBeDisabled();
  return ready;
}

export async function acknowledge(page) {
  await editor(page)
    .getByRole('checkbox', {
      name: 'Confirmo el resultado y el plazo revisados',
      exact: true,
    })
    .check();
}

export async function submit(page) {
  const pending = responseTo(page, `${endpoint}/submit`);
  await acknowledge(page);
  await editor(page)
    .getByRole('button', { name: 'Confirmar resultado y plazo', exact: true })
    .click();
  const response = await pending;
  expect(response.status()).toBe(201);
  const record = await response.json();
  await expect(editor(page)).toHaveCount(0);
  return record;
}

export async function verifyExact(call, ready, record) {
  expect(record.command).toEqual(ready.command);
  expect(record.result.revision).toBe(1);
  expect(record.deadline.revision).toBe(1);
  expect(record.result.recorded_by.id).toBe(scenario.owner.id);
  expect(record.deadline.responsible.id).toBe(scenario.owner.id);
  expect(record.deadline.calculation.result).toEqual(ready.deadline.result);
  expect(record.origin).toMatchObject({
    case_id: scenario.case.id,
    hearing_id: scenario.hearing.id,
    result_id: record.result.id,
    result_revision: 1,
    deadline_id: record.deadline.id,
    deadline_revision: 1,
    result_operation_id: ready.command.result.operation_id,
    deadline_operation_id: ready.command.deadline.operation_id,
    review_digest: ready.review_digest,
    capture_digest: record.capture_digest,
  });
  expect(record.origin.source_event.sequence).toMatch(/^[1-9][0-9]*$/);
  expect(record.deadline.calculation.material.source.reference).toEqual({
    family: 'hearing_result',
    hearing_id: scenario.hearing.id,
    result_id: record.result.id,
    revision: 1,
    agreement_id: null,
  });
  expect(
    await call(`${scope}/hearings/${scenario.hearing.id}/results/${record.result.id}/revisions/1`),
  ).toEqual(record.result);
  expect(await call(`${scope}/deadlines/${record.deadline.id}/revisions/1`)).toEqual(
    record.deadline,
  );
}
