import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  deadlineFixture,
  caseId,
} from './deadline-editor-helpers.mjs';
import { id, instant, known, hash } from '../fixtures/deadline-unit.mjs';
import { v1Record as detail } from '../fixtures/deadline-v2-unit.mjs';
function examples() {
  const daily = deadlineFixture(),
    monthly = deadlineFixture(),
    hourly = deadlineFixture();
  const date = { precision: 'date', year: 2026, month: 1, day: 31, offset_seconds: null };
  daily.command.deadline_id = id(30);
  daily.definition.title = 'Computo por calendario con fecha pendiente';
  const days = {
      kind: 'days',
      quantity: 2,
      inclusion: 'on_anchor',
      basis: 'calendar_countable',
      final_day: 'preserve',
    },
    block = { kind: 'unresolved_calendar_date', date: '2026-02-01' };
  daily.calculation.result = {
    requirement: { kind: 'source_field', field: 'resolution_issued_at' },
    trigger_outcome: { kind: 'extracted', at: date },
    rule: days,
    arithmetic: {
      rule: days,
      anchor: date,
      outcome: { kind: 'blocked', block },
      trace: [
        {
          kind: 'counted_days',
          count: {
            first_included: '2026-01-31',
            quantity: 2,
            accumulated: 1,
            outcome: { kind: 'unresolved', date: '2026-02-01' },
            trace: [
              {
                day: {
                  date: '2026-01-31',
                  origin: { kind: 'weekly_pattern', weekday: 6 },
                  classification: 'countable',
                  explanation: 'Dia computable segun la regla declarada.',
                  source_ids: [id(50)],
                },
                accumulated: 1,
              },
              {
                day: {
                  date: '2026-02-01',
                  origin: { kind: 'exception', id: id(51) },
                  classification: 'unresolved',
                  explanation: 'Incidencia pendiente de calificacion.',
                  source_ids: [id(50)],
                },
                accumulated: 1,
              },
            ],
          },
        },
      ],
    },
    due_at: null,
    blocks: [{ kind: 'arithmetic', block }],
  };
  monthly.command.deadline_id = id(31);
  monthly.definition.title = 'Mes civil sin dia homologo';
  const months = { kind: 'civil_months', quantity: 1, final_day: 'preserve' },
    missing = { kind: 'missing_homologous_day', year: 2026, month: 2, requested_day: 31 };
  monthly.calculation.result = {
    requirement: { kind: 'source_field', field: 'resolution_issued_at' },
    trigger_outcome: { kind: 'extracted', at: date },
    rule: months,
    arithmetic: {
      rule: months,
      anchor: date,
      outcome: { kind: 'blocked', block: missing },
      trace: [
        {
          kind: 'civil_months',
          anchor: '2026-01-31',
          quantity: 1,
          target_year: 2026,
          target_month: 2,
          requested_day: 31,
          candidate: null,
        },
      ],
    },
    due_at: null,
    blocks: [{ kind: 'arithmetic', block: missing }],
  };
  hourly.command.deadline_id = id(32);
  hourly.definition.title = 'Horas con representacion temporal completa';
  const hours = { kind: 'elapsed_hours', quantity: 24 },
    at = {
      precision: 'second',
      year: 2026,
      month: 1,
      day: 1,
      hour: 0,
      minute: 0,
      second: 0,
      offset_seconds: 0,
    };
  const start = { ...instant(), offset_seconds: 93599 },
    end = { unix_seconds: 1767312000, nanosecond: 123456789, offset_seconds: 0 };
  hourly.calculation.result = {
    requirement: { kind: 'source_field', field: 'resolution_issued_at' },
    trigger_outcome: { kind: 'extracted', at },
    rule: hours,
    arithmetic: {
      rule: hours,
      anchor: at,
      outcome: { kind: 'instant_candidate', instant: end },
      trace: [{ kind: 'elapsed_hours', start, quantity: 24, candidate: end }],
    },
    due_at: end,
    blocks: [],
  };
  return [daily, monthly, hourly].map((value, index) => {
    const profileId = id(40 + index),
      sourceId = id(60 + index),
      reference = { family: 'resolution', id: sourceId, revision: 1 };
    value.definition.profile.id = profileId;
    value.calculation.profile.id = profileId;
    value.calculation.profile.title = [
      'Dias segun calendario',
      'Meses civiles declarados',
      'Horas declaradas',
    ][index];
    value.calculation.profile.href = `/api/v1/cases/${caseId}/deadline-profiles/${profileId}/revisions/1`;
    value.definition.input.selection.source = known(reference);
    const source = {
      case_id: caseId,
      reference,
      values_digest: hash('1'),
      sources_digest: hash('2'),
      submission_digest: hash('3'),
      status: 'recorded',
      href: `/api/v1/cases/${caseId}/resolutions/${sourceId}/revisions/1`,
    };
    value.calculation.material.source = source;
    value.calculation.material.source_head = structuredClone(source);
    if (index === 0) {
      value.definition.input.calendar = { id: id(70), revision: 1 };
      value.calculation.material.calendar = {
        id: id(70),
        revision: 1,
        values_digest: hash('4'),
        submission_digest: hash('5'),
        status: 'published',
        title: 'Calendario declarado',
        href: `/api/v1/judicial-calendars/${id(70)}/revisions/1`,
      };
      value.calculation.material.calendar_head = structuredClone(
        value.calculation.material.calendar,
      );
    }
    return detail(value);
  });
}
for (const width of [1440, 390])
  test(`daily monthly and hourly deadline results remain readable at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const rows = examples();
    await setupDeadlines(page, { deadlines: rows });
    await openDeadlines(page);
    for (const [index, row] of rows.entries()) {
      await page.getByRole('button', { name: `Consultar plazo ${row.id}`, exact: true }).click();
      const panel = page.getByRole('region', { name: 'Detalle de plazo', exact: true });
      await expect(panel).toContainText(row.definition.title);
      await panel.getByText('Ver pasos del c\u00f3mputo (1)', { exact: true }).click();
      if (index === 0) {
        await expect(panel.getByRole('cell', { name: 'Sin resolver', exact: true })).toBeVisible();
        await expect(panel).toContainText('El calendario deja sin resolver la fecha 2026-02-01.');
        await expect(panel).toContainText('Incidencia pendiente de calificacion.');
        const heading = await panel
          .getByRole('columnheader', { name: 'Clasificaci\u00f3n', exact: true })
          .boundingBox();
        expect(heading.width).toBeGreaterThanOrEqual(100);
      } else if (index === 1) {
        await expect(panel).toContainText('El d\u00eda 31 no existe en 2026-02');
        await expect(panel).toContainText('El d\u00eda requerido no existe en el mes de destino.');
        await expect(panel).not.toContainText('2026-02-28');
      } else {
        await expect(panel).toContainText('2026-01-02 00:00:00.123456789 / UTC+00:00');
        await expect(panel).toContainText('UTC+25:59:59');
      }
      await panel.getByText('Autor y recibo de esta captura', { exact: true }).click();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
      await page.evaluate(() => {
        document.activeElement?.blur();
        window.scrollTo(0, 0);
      });
      await page.screenshot({
        path: `../output/deadline-workspace/deadline-${['daily', 'monthly', 'hourly'][index]}-${width}.png`,
        fullPage: true,
      });
      if (index === 0 && width === 390) {
        const region = panel.getByRole('region', {
          name: 'Detalle de d\u00edas computados',
          exact: true,
        });
        const foundation = await region
          .getByRole('columnheader', { name: 'Fundamento registrado', exact: true })
          .boundingBox();
        expect(foundation.width).toBeGreaterThanOrEqual(140);
        await region.evaluate((element) => {
          element.scrollLeft = element.scrollWidth;
        });
        await region.screenshot({
          path: '../output/deadline-workspace/deadline-daily-390-foundation.png',
        });
      }
    }
  });
