import { expect } from '@playwright/test';
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de plazo', exact: true });

export async function fillContextualDeadline(page, scenario, actor, title) {
  const form = editor(page);
  await form.getByLabel('T\u00edtulo del plazo', { exact: true }).fill(title);
  await form.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
  await form.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
  await form.getByRole('button', { name: 'Usar este acto', exact: true }).click();
  await form.getByRole('button', { name: 'Elegir perfil exacto', exact: true }).click();
  await form
    .getByRole('button', {
      name: `Revisiones de ${scenario.profile.definition.title}`,
      exact: true,
    })
    .click();
  await form
    .getByRole('button', { name: 'Consultar perfil revisi\u00f3n 1 (publicado)', exact: true })
    .click();
  await form.getByRole('button', { name: 'Usar este perfil exacto', exact: true }).click();
  await form
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('follow');
  await form.getByRole('button', { name: 'Elegir responsable', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Elegir responsable', exact: true });
  await expect(picker).toHaveAttribute('aria-busy', 'false');
  const responsible = form.getByRole('button', {
    name: `Elegir responsable ${actor.email}`,
    exact: true,
  });
  for (let index = 0; (await responsible.count()) === 0 && index < 10; index += 1) {
    const next = form.getByRole('button', { name: 'Siguientes responsables', exact: true });
    if (!(await next.isEnabled()))
      throw new Error('Contextual deadline responsible is not available');
    const pending = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname ===
          `/api/v1/cases/${scenario.case.id}/deadlines/responsibles` &&
        response.request().method() === 'GET',
    );
    await next.click();
    expect((await pending).status()).toBe(200);
    await expect(picker).toHaveAttribute('aria-busy', 'false');
  }
  await responsible.click();
  await form
    .getByRole('combobox', { name: 'Tipo de fuente', exact: true })
    .selectOption('resolution');
  await form.getByRole('button', { name: 'Elegir fuente exacta', exact: true }).click();
  await form
    .getByRole('button', {
      name: `Consultar revisiones de resolucion ${scenario.source.id}`,
      exact: true,
    })
    .click();
  await form
    .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await form
    .getByRole('button', { name: 'Vincular esta revisi\u00f3n de resoluci\u00f3n', exact: true })
    .click();
  await form
    .getByRole('combobox', { name: 'Cuando cambie la fuente', exact: true })
    .selectOption('fixed');
  await form.getByRole('button', { name: 'Elegir calendario exacto', exact: true }).click();
  await form
    .getByRole('button', {
      name: `Revisiones de ${scenario.calendar.values.scope.title}`,
      exact: true,
    })
    .click();
  await form
    .getByRole('button', { name: 'Consultar calendario revisi\u00f3n 1 (publicado)', exact: true })
    .click();
  await form.getByRole('button', { name: 'Usar este calendario exacto', exact: true }).click();
  await form
    .getByRole('combobox', { name: 'Cuando cambie el calendario', exact: true })
    .selectOption('fixed');
  await form
    .getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })
    .fill('Supuesto contextual sintetico sin afirmar validez juridica');
  await form
    .getByLabel('Localizador de aplicabilidad', { exact: true })
    .fill('Fuente contextual, pagina 1');
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
    .fill('Declaracion contextual');
}

export async function queryContextualAgenda(page, scenario, deadline) {
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption('day');
  await page.getByLabel('Fecha de referencia', { exact: true }).fill(scenario.date);
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('+00:00');
  const pending = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === '/api/v1/agenda' &&
      response.request().method() === 'GET',
  );
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(200);
  const pageData = await response.json();
  const matches = pageData.items.filter(
    (row) => row.kind === 'deadline' && row.deadline.id === deadline.id,
  );
  expect(matches).toHaveLength(1);
  expect(matches[0].at).toEqual(scenario.dueAt);
  expect(matches[0].deadline.operational).toMatchObject({
    freshness: 'current',
    due_at: scenario.dueAt,
    checked_at: pageData.checked_at,
  });
  await expect(page.getByRole('region', { name: 'Agenda combinada', exact: true })).toHaveAttribute(
    'aria-busy',
    'false',
  );
  const open = page.getByRole('button', { name: `Consultar plazo ${deadline.id}`, exact: true });
  await expect(open).toBeVisible();
  const exact = page.waitForResponse(
    (result) =>
      new URL(result.url()).pathname ===
        `/api/v1/cases/${scenario.case.id}/deadlines/${deadline.id}/revisions/1` &&
      result.request().method() === 'GET',
  );
  await open.click();
  const read = await exact;
  expect(read.status()).toBe(200);
  expect(await read.json()).toEqual(deadline);
  await expect(page.getByRole('region', { name: 'Detalle de plazo', exact: true })).toBeVisible();
}
