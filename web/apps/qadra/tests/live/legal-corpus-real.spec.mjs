import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';

const corpus = fixture.legalCorpus;
const positiveKinds = [
  'initial',
  'extension',
  'oral',
  'written',
  'control_order',
  'control_sentence',
  'withdrawal',
  'trial_sentence',
];
const detail = (page) => page.getByRole('region', { name: 'Detalle de plazo', exact: true });
const list = (page) => page.getByRole('region', { name: 'Plazos registrados', exact: true });

async function openRecord(page, record) {
  const button = list(page).getByRole('button', {
    name: `Consultar plazo ${record.row.id}`,
    exact: true,
  });
  if (!(await button.count())) {
    await list(page).getByRole('button', { name: 'Actualizar plazos', exact: true }).click();
    await expect(list(page)).toHaveAttribute('aria-busy', 'false');
  }
  for (let pageNumber = 0; !(await button.count()) && pageNumber < 10; pageNumber++) {
    const next = list(page).getByRole('button', { name: 'Plazos siguientes', exact: true });
    await expect(next).toBeEnabled();
    await next.click();
    await expect(list(page)).toHaveAttribute('aria-busy', 'false');
  }
  const pending = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname ===
        `/api/v1/cases/${corpus.scenario.case_id}/deadlines/${record.row.id}` &&
      response.request().method() === 'GET',
  );
  await button.click();
  const response = await pending;
  expect(response.status()).toBe(200);
  const observed = await response.json();
  expect(observed.calculation).toEqual(record.row.calculation);
  await expect(
    detail(page).getByRole('heading', {
      name: record.row.definition.title,
      exact: true,
    }),
  ).toBeVisible();
  return observed;
}

async function inspectProfile(page, record) {
  const profile = corpus.scenario.profiles[record.kind];
  const section = detail(page).getByRole('region', {
    name: 'Perfil exacto del plazo',
    exact: true,
  });
  await section.getByRole('button', { name: 'Consultar perfil capturado', exact: true }).click();
  await expect(
    section.getByRole('heading', {
      name: `${profile.definition.title} / Revisi\u00f3n 1`,
      exact: true,
    }),
  ).toBeVisible();
  for (const reference of profile.definition.references) {
    const link = section.getByRole('link', {
      name: `Abrir referencia: ${reference.title}`,
      exact: true,
    });
    await expect(link).toBeVisible();
    await expect(link).toHaveAttribute('href', reference.official_url);
  }
  const completion = profile.definition.completion;
  if (completion.kind === 'weekly_civil_cutoff') {
    await expect(section).toContainText('Cierre semanal del canal elegido');
    await expect(section).toContainText('no una caducidad general');
    for (const [day, hour] of [
      ['Lunes', '15:00:00'],
      ['Martes', '15:00:00'],
      ['Mi\u00e9rcoles', '15:00:00'],
      ['Jueves', '15:00:00'],
      ['Viernes', '14:00:00'],
      ['S\u00e1bado', 'Sin cobertura'],
      ['Domingo', 'Sin cobertura'],
    ])
      await expect(
        section.getByRole('listitem').filter({ hasText: `${day}: ${hour}` }),
      ).toBeVisible();
    await expect(section).toContainText(completion.channel);
    await expect(section).toContainText(`${completion.from} / ${completion.through}`);
  } else if (record.kind === 'oral') {
    await expect(section).toContainText('Instante del fin declarado de audiencia.');
  }
}

async function inspectCalendar(page, record) {
  const material = record.row.calculation.material;
  await detail(page)
    .getByText('Consultar insumos y fuentes de esta revisi\u00f3n', {
      exact: true,
    })
    .click();
  const inputs = detail(page).getByRole('region', {
    name: 'Insumos capturados del plazo',
    exact: true,
  });
  if (!material.calendar) {
    await expect(inputs).toContainText('Sin calendario seleccionado.');
    return;
  }
  const calendar = inputs.getByRole('article', { name: 'Calendario seleccionado', exact: true });
  await expect(calendar).toContainText(`${material.calendar.title} / Revisi\u00f3n 1`);
  await calendar
    .getByRole('button', { name: 'Consultar calendario seleccionado', exact: true })
    .click();
  await expect(calendar).toContainText(corpus.scenario.calendar.values.scope.title);
  for (const source of corpus.scenario.calendar.values.sources) {
    const link = calendar.getByRole('link', {
      name: `Abrir referencia: ${source.title}`,
      exact: true,
    });
    await expect(link).toBeVisible();
    await expect(link).toHaveAttribute('href', source.official_url);
  }
  if (record.variant === 'written_holiday') {
    await calendar
      .getByText('Patr\u00f3n semanal y excepciones de esta revisi\u00f3n', {
        exact: true,
      })
      .click();
    await expect(calendar).toContainText('2026-11-02');
  }
}

for (const [width, recoveryIndex] of [
  [1440, 0],
  [390, 1],
])
  test(`real legal corpus preserves eight results, calendar and blocked effects at ${width}px`, async ({
    page,
  }, info) => {
    test.setTimeout(180000);
    const errors = [],
      writes = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: width === 390 ? 844 : 1000 });
    expect((await page.goto('/')).status()).toBe(200);
    await loginAs(page, corpus.owner, recoveryIndex);
    const casePath = `/api/v1/cases/${corpus.scenario.case_id}/`;
    page.on('request', (request) => {
      if (new URL(request.url()).pathname.startsWith(casePath) && request.method() !== 'GET')
        writes.push(request.method());
    });
    await openCase(page, corpus.scenario.case);
    await page.getByRole('link', { name: 'Plazos', exact: true }).click();
    await expect(list(page)).toHaveAttribute('aria-busy', 'false');
    const positives = corpus.records.filter((record) => record.variant === 'positive');
    expect(positives.map((record) => record.kind).sort()).toEqual([...positiveKinds].sort());
    const holiday = corpus.records.find((record) => record.variant === 'written_holiday');
    expect(holiday).toBeTruthy();
    for (const record of [...positives, holiday]) {
      const observed = await openRecord(page, record);
      const result = observed.calculation.result;
      expect(result.blocks).toEqual([]);
      expect(result.due_at).toEqual({
        unix_seconds: Date.parse(record.expected) / 1000,
        nanosecond: 0,
        offset_seconds: 0,
      });
      const calculation = detail(page).getByRole('region', {
        name: 'Resultado del plazo',
        exact: true,
      });
      const utc = new Date(record.expected).toISOString().slice(0, 19).replace('T', ' ');
      await expect(calculation.locator('.deadline-due')).toHaveText(`${utc}.000000000 / UTC+00:00`);
      if (result.arithmetic.outcome.kind === 'civil_candidate')
        await expect(calculation).toContainText(`Fecha candidata: ${record.expected.slice(0, 10)}`);
      await inspectProfile(page, record);
      await inspectCalendar(page, record);
      if (record.kind === 'written' && record.variant === 'positive')
        await page.screenshot({
          path: info.outputPath('weekly-channel-profile.png'),
          fullPage: true,
        });
    }
    const blocked = corpus.records.find((record) => record.variant === 'missing_effect');
    expect(blocked).toBeTruthy();
    const observed = await openRecord(page, blocked);
    expect(observed.calculation.result.due_at).toBeNull();
    expect(observed.calculation.result.blocks).toContainEqual({
      kind: 'trigger',
      block: { kind: 'absent_field', field: 'notification_stated_effect_at' },
    });
    await expect(detail(page)).toContainText('C\u00e1lculo bloqueado');
    await expect(detail(page)).toContainText('Sin vencimiento calculado.');
    await expect(detail(page)).toContainText(
      'La fuente no contiene Efecto declarado de la notificaci\u00f3n.',
    );
    await expect(detail(page).locator('.deadline-due')).toHaveCount(0);
    await page.screenshot({
      path: info.outputPath('missing-notification-effect.png'),
      fullPage: true,
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    expect(writes).toEqual([]);
    expect(errors).toEqual([]);
  });
