import { expect } from '@playwright/test';
import {
  caseRoute,
  measurePanel,
  administrationEditor,
  administrationDetail,
  reviewAction,
  responseTo,
  confirmation,
} from './precautionary-hearings-real-helpers.mjs';

export async function rectifyWithLostResponse(page, scenario, before, action) {
  await measurePanel(page)
    .getByRole('button', { name: `Consultar medida ${before.reference.id}`, exact: true })
    .click();
  await measurePanel(page)
    .getByRole('button', {
      name: action === 'correct' ? 'Rectificar registro' : 'Corregir identidad registrada',
      exact: true,
    })
    .click();
  const form = administrationEditor(page);
  await form
    .getByLabel('Motivo de rectificacion', { exact: true })
    .fill('Rectificacion administrativa declarada en soporte');
  if (action === 'correct') {
    await form
      .getByLabel('Condiciones', { exact: true })
      .fill('Presentarse en la sede rectificada del soporte');
    await form
      .getByLabel('Declaracion de vigencia', { exact: true })
      .fill('Vigencia textual rectificada sin nuevo termino');
    await form
      .getByLabel('Texto de supervision', { exact: true })
      .fill('Texto rectificado sin autoridad supervisora declarada');
  } else {
    await form.getByRole('button', { name: 'Elegir sujeto', exact: true }).click();
    const picker = form.getByRole('region', { name: 'Elegir identidad existente', exact: true });
    await picker
      .getByRole('button', {
        name: 'Consultar identidad: Persona sustituta declarada',
        exact: true,
      })
      .click();
    await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  }
  const base = '/api/v1' + caseRoute(scenario) + '/measure-administrative-operations';
  const prepared = await reviewAction(
    page,
    form,
    base,
    'rectificacion',
    'Reconozco la rectificacion y su alcance administrativo',
  );
  expect(prepared.command.target).toEqual(before.reference);
  expect(prepared.command.action.kind).toBe(action);
  let operation, submitted, resolve, reject;
  const intercepted = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  const endpoint = `**${base}/submit`;
  await page.route(endpoint, async (route) => {
    try {
      submitted = route.request().postDataJSON();
      const response = await route.fetch({ maxRedirects: 0 });
      expect(response.status()).toBe(201);
      expect(response.headers()['cache-control']).toBe('no-store');
      operation = await response.json();
      await route.abort('failed');
      resolve();
    } catch (error) {
      reject(error);
      await route.abort('failed').catch(() => {});
    }
  });
  try {
    await Promise.all([
      form.getByRole('button', { name: 'Confirmar rectificacion', exact: true }).click(),
      intercepted,
    ]);
    await expect(
      form.getByRole('heading', { name: 'Resultado incierto', exact: true }),
    ).toBeVisible();
    expect(submitted).toEqual(confirmation(prepared));
    expect(operation.capture.review).toEqual(prepared);
    expect(operation.record_history).toEqual(before.record_history);
    const reading = responseTo(page, `${base}/${prepared.command.operation_id}`);
    await form.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
    const response = await reading;
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toBe('no-store');
    expect(await response.json()).toEqual(operation);
    await expect(form).toHaveCount(0);
    await expect(administrationDetail(page)).toContainText(prepared.command.operation_id);
    await expect(administrationDetail(page)).toContainText(prepared.command.reason);
  } finally {
    await page.unroute(endpoint);
  }
  return { prepared, operation, submitted };
}
