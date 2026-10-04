import { expect } from '@playwright/test';
import { navigate } from '../case-administration-workflow.mjs';
import { scenario, responseTo } from './hearing-derived-deadline-helpers.mjs';

export async function observeAgenda(page, call, record, timed) {
  const date = new Date(scenario.dueAt.unix_seconds * 1000).toISOString().slice(0, 10);
  const from = `${date}T00:00:00Z`;
  const until = new Date(Date.parse(from) + 86400000).toISOString().replace('.000Z', 'Z');
  const query = new URLSearchParams({ from, until, kind: 'deadline', limit: '100' });
  const result = await call(`/api/v1/agenda?${query}`);
  expect(result.complete).toBe(true);
  expect(result.next_cursor).toBeNull();
  const rows = result.items.filter(
    (row) => row.kind === 'deadline' && row.deadline.id === record.deadline.id,
  );
  expect(rows).toHaveLength(timed ? 1 : 0);
  if (!timed) return result;
  expect(rows[0].at).toEqual(scenario.dueAt);
  expect(rows[0].deadline.operational).toEqual({
    freshness: 'current',
    checked_at: result.checked_at,
    changed_dependencies: [],
    due_at: scenario.dueAt,
  });
  await navigate(page, 'Agenda');
  const agenda = page.getByRole('region', { name: 'Agenda combinada', exact: true });
  await expect(agenda).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption('day');
  await page.getByLabel('Fecha de referencia', { exact: true }).fill(date);
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('+00:00');
  const pending = responseTo(page, '/api/v1/agenda', 'GET');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  expect((await pending).status()).toBe(200);
  await expect(agenda).toHaveAttribute('aria-busy', 'false');
  await expect(
    agenda.getByRole('button', {
      name: `Consultar plazo ${record.deadline.id}`,
      exact: true,
    }),
  ).toBeVisible();
  return result;
}

export async function observeAlerts(page, call, record, timed) {
  let result, alert;
  const read = async () => {
    result = await call('/api/v1/alerts?limit=100&read=all&state=active');
    alert = result.alerts.find(
      (row) =>
        row.subject.kind === 'deadline' &&
        row.subject.id === record.deadline.id &&
        row.kind.kind === 'upcoming',
    );
    return !!alert;
  };
  if (!timed) {
    await read();
    expect(
      result.alerts.filter(
        (row) =>
          row.subject.kind === 'deadline' &&
          row.subject.id === record.deadline.id &&
          ['upcoming', 'overdue_unattended'].includes(row.kind.kind),
      ),
    ).toEqual([]);
    return result;
  }
  await expect.poll(read, { timeout: 8000, intervals: [250, 500, 1000] }).toBe(true);
  expect(alert).toMatchObject({
    recipient_id: scenario.owner.id,
    subject: { kind: 'deadline', case_id: scenario.case.id, id: record.deadline.id },
    origin: { revision: 1, evidence_digest: record.deadline.receipt.capture_digest },
    kind: { kind: 'upcoming', lead_hours: 24, activity_at: scenario.dueAt },
    state: { kind: 'active' },
    email: { kind: 'disabled' },
  });
  await navigate(page, 'Alertas');
  await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
  const card = page.locator(`[data-alert-id="${alert.id}"]`);
  await expect(card).toContainText(record.deadline.definition.title);
  await expect(card).toContainText('24 horas antes');
  await expect(card).toContainText('Correo deshabilitado');
  return result;
}
