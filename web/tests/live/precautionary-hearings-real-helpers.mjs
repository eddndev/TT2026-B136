import { expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { openCase, responseTo } from './procedural-resources-helpers.mjs';
export { openCase, responseTo };
export const accounts = fixture.precautionaryHearings;
export const hearingEditor = (page) =>
  page.getByRole('region', { name: 'Convocatoria cautelar', exact: true });
export const hearingDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia cautelar', exact: true });
export const decisionEditor = (page) =>
  page.getByRole('region', { name: 'Decision cautelar', exact: true });
export const decisionDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de decision cautelar', exact: true });
export const administrationEditor = (page) =>
  page.getByRole('region', { name: 'Rectificacion de medida', exact: true });
export const administrationDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de rectificacion', exact: true });
export const measurePanel = (page) =>
  page.getByRole('region', { name: 'Registros de medidas', exact: true });
export const caseRoute = (scenario) => `/cases/${scenario.case.id}`;
export const hearingExact = (scenario, hearing) =>
  '/api/v1' +
  caseRoute(scenario) +
  `/precautionary-hearings/${hearing.capture.review.command.hearing_id}/revisions/1`;
export const confirmation = (review) => ({
  command: review.command,
  expected_submission_digest: review.submission_digest,
  expected_review_digest: review.review_digest,
});

export async function withSession(page, actor, run) {
  const verified = responseTo(page, '/api/v1/auth/mfa/recovery', 'POST');
  await loginAs(page, actor, 0);
  const session = await (await verified).json();
  let token = session.access_token;
  delete session.access_token;
  async function call(method, path, data, expected = 200) {
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      redirect: 'error',
      headers: {
        Authorization: `Bearer ${token}`,
        ...(data === undefined ? {} : { 'Content-Type': 'application/json' }),
      },
      body: data === undefined ? undefined : JSON.stringify(data),
    });
    expect(response.status, `${method} ${path}`).toBe(expected);
    return response.status === 204 ? null : response.json();
  }
  try {
    return await run(call);
  } finally {
    try {
      await call('POST', '/auth/logout', undefined, 204);
    } finally {
      token = null;
    }
  }
}

export async function capture(page, testInfo, name) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.evaluate(async () => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
}

export async function chooseSupport(form, support) {
  await form.getByRole('button', { name: 'Elegir soporte', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker
    .getByRole('button', {
      name: `${support.name} / versi\u00f3n actual ${support.version}`,
      exact: true,
    })
    .click();
  await picker
    .getByRole('button', {
      name: `Versi\u00f3n ${support.version} / ${support.name} / Actual al consultar`,
      exact: true,
    })
    .click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
}

export async function reviewAction(page, form, path, noun, acknowledgement) {
  const waiting = responseTo(page, `${path}/prepare`, 'POST');
  await form.getByRole('button', { name: `Revisar ${noun}`, exact: true }).click();
  const response = await waiting;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  const prepared = await response.json(),
    review = prepared.review ?? prepared;
  expect(review.submission_digest).toMatch(/^[a-f0-9]{64}$/);
  expect(review.review_digest).toMatch(/^[a-f0-9]{64}$/);
  await expect(form.getByRole('button', { name: `Confirmar ${noun}`, exact: true })).toBeDisabled();
  await form.getByRole('checkbox', { name: acknowledgement, exact: true }).check();
  return prepared;
}

async function submitAction(page, form, path, noun, review) {
  const waiting = responseTo(page, `${path}/submit`, 'POST');
  await form.getByRole('button', { name: `Confirmar ${noun}`, exact: true }).click();
  const response = await waiting;
  expect(response.status()).toBe(201);
  expect(response.headers()['cache-control']).toBe('no-store');
  expect(response.request().postDataJSON()).toEqual(confirmation(review));
  const operation = await response.json();
  await expect(form).toHaveCount(0);
  return operation;
}

export async function scheduleHearing(page, scenario) {
  await openCase(page, scenario.case);
  await page.getByRole('link', { name: 'Audiencias', exact: true }).click();
  await page.getByRole('button', { name: 'Programar audiencia cautelar', exact: true }).click();
  const form = hearingEditor(page);
  const seconds = Math.floor(Date.now() / 1000) + 36 * 3600;
  const local = new Date((seconds - 6 * 3600) * 1000).toISOString();
  const scheduledAt = local.slice(0, 19) + '-06:00';
  await form
    .getByRole('combobox', { name: /Prop[o\u00f3]sito/, exact: true })
    .selectOption('imposition');
  await form.getByLabel('Fecha', { exact: true }).fill(local.slice(0, 10));
  await form.getByLabel('Hora', { exact: true }).fill(local.slice(11, 19));
  await form.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await form
    .getByLabel('Sede o enlace', { exact: true })
    .fill(`Sala cautelar ${scenario.case.reference}`);
  await form.getByLabel('Nota', { exact: true }).fill('Convocatoria sintetica con fuentes exactas');
  await form
    .getByLabel(/Base de se[n\u00f1]alamiento/, { exact: true })
    .fill('Senalamiento expresamente comunicado');
  await form.getByLabel('Localizador', { exact: true }).fill('Pagina 1');
  await chooseSupport(form, scenario.support);
  await form.getByRole('button', { name: 'Elegir participante', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Seleccionar participante exacto', exact: true });
  await picker
    .getByRole('button', { name: 'Consultar ficha: Persona declarada', exact: true })
    .click();
  await picker.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
  const path = '/api/v1' + caseRoute(scenario) + '/precautionary-hearings';
  const prepared = await reviewAction(
    page,
    form,
    path,
    'convocatoria',
    'Reconozco la convocatoria y las fuentes seleccionadas',
  );
  const operation = await submitAction(page, form, path, 'convocatoria', prepared);
  await expect(hearingDetail(page)).toBeVisible();
  return { prepared, operation, seconds, scheduledAt };
}

export async function declareMeasure(page, scenario, hearing, anchorKind) {
  await openCase(page, scenario.case);
  await page.getByRole('link', { name: 'Medidas cautelares', exact: true }).click();
  await page.getByRole('button', { name: 'Registrar decision cautelar', exact: true }).click();
  const form = decisionEditor(page);
  await form.getByLabel('Autoridad', { exact: true }).fill('Juzgado declarado en soporte');
  await form
    .getByLabel(/Justificaci[o\u00f3]n/, { exact: true })
    .fill('Imposicion expresamente declarada en soporte');
  await form
    .getByRole('combobox', { name: /Precisi[o\u00f3]n de decisi[o\u00f3]n/, exact: true })
    .selectOption('unknown');
  await form
    .getByLabel(/Motivo de tiempo desconocido de decisi[o\u00f3]n/, { exact: true })
    .fill('El soporte no declara el momento exacto');
  await form.getByLabel('Localizador', { exact: true }).fill('Pagina 1');
  await chooseSupport(form, scenario.support);
  await form
    .getByRole('combobox', { name: 'Vinculo de audiencia', exact: true })
    .selectOption(anchorKind);
  if (anchorKind === 'initial') {
    const picker = form.getByRole('region', {
      name: 'Elegir programaci\u00f3n hist\u00f3rica',
      exact: true,
    });
    await picker
      .getByRole('button', {
        name: `Consultar programaci\u00f3n ${scenario.initial.id}`,
        exact: true,
      })
      .click();
    await picker
      .getByRole('button', { name: 'Usar programaci\u00f3n revisi\u00f3n 1', exact: true })
      .click();
  } else {
    const picker = form.getByRole('region', {
      name: 'Elegir audiencia cautelar de origen',
      exact: true,
    });
    await expect(picker.getByRole('button', { name: /^Consultar convocatoria / })).toHaveCount(1);
    await picker.getByRole('button', { name: /^Consultar convocatoria / }).click();
    await picker
      .getByRole('button', { name: 'Usar audiencia cautelar revision 1', exact: true })
      .click();
  }
  const effect = form.getByRole('group', { name: 'Efecto 1', exact: true });
  await effect
    .getByRole('combobox', { name: /Acci[o\u00f3]n de medida 1/, exact: true })
    .selectOption('impose');
  await effect.getByRole('button', { name: 'Elegir sujeto', exact: true }).click();
  const subjectPicker = effect.getByRole('region', {
    name: 'Elegir identidad existente',
    exact: true,
  });
  await subjectPicker
    .getByRole('button', { name: 'Consultar identidad: Persona declarada', exact: true })
    .click();
  await subjectPicker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  await effect
    .getByRole('combobox', { name: 'Clase de medida', exact: true })
    .selectOption('periodic_appearance');
  await effect
    .getByLabel('Condiciones', { exact: true })
    .fill('Presentarse cada viernes segun soporte');
  await effect
    .getByRole('combobox', { name: /Precisi[o\u00f3]n de inicio de vigencia/, exact: true })
    .selectOption('unknown');
  await effect
    .getByLabel('Motivo de tiempo desconocido de inicio de vigencia', { exact: true })
    .fill('No consta inicio de vigencia');
  await effect
    .getByLabel(/Declaraci[o\u00f3]n de vigencia/, { exact: true })
    .fill('Vigencia declarada sin termino conocido');
  await effect
    .getByLabel(/Motivo de supervisi[o\u00f3]n desconocida/, { exact: true })
    .fill('No consta autoridad supervisora');
  const path = '/api/v1' + caseRoute(scenario) + '/measure-decisions';
  const prepared = await reviewAction(
    page,
    form,
    path,
    'decision',
    'Reconozco la decision y las fuentes seleccionadas',
  );
  const operation = await submitAction(page, form, path, 'decision', prepared.review);
  await expect(decisionDetail(page)).toBeVisible();
  const anchor = prepared.review.command.anchor;
  expect(anchor).toEqual(
    anchorKind === 'initial'
      ? {
          kind: 'initial',
          hearing_id: scenario.initial.id,
          revision: 1,
          values_digest: scenario.initial.values_digest,
          submission_digest: scenario.initial.receipt.submission_digest,
        }
      : {
          kind: 'precautionary',
          hearing_id: hearing.capture.review.command.hearing_id,
          revision: 1,
          capture_digest: hearing.capture.capture_digest,
        },
  );
  return { prepared, operation };
}
