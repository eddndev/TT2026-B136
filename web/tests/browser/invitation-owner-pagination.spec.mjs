import { test, expect } from '@playwright/test';
import { setupMembers, enterDirectory } from './members-helpers.mjs';

const region = (page) =>
  page.getByRole('region', { name: 'Invitaciones al despacho', exact: true });
const summary = (id, email) => ({
  id,
  operation_id: id,
  revision: '1',
  email,
  role: 'paralegal',
  status: 'pending',
  delivery: 'pending',
  issued_at: '2026-10-11T00:00:00Z',
  expires_at: '2026-10-12T00:00:00Z',
  accepted_at: null,
});

test('creating an invitation before loading the next page keeps each row once', async ({
  page,
}) => {
  await setupMembers(page);
  const pageErrors = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  const first = Array.from({ length: 20 }, (_, index) =>
    summary(
      `00000000-0000-0000-0000-${String(index + 1).padStart(12, '0')}`,
      `pending-${index + 1}@example.test`,
    ),
  );
  const cursor = first.at(-1).id;
  const last = summary('ffffffff-ffff-4fff-bfff-ffffffffffff', 'last@example.test');
  let created = null;
  const cursors = [];
  await page.route('**/api/v1/member-invitations**', async (route) => {
    const request = route.request();
    if (request.method() === 'POST') {
      const data = request.postDataJSON();
      created = { ...summary(data.operation_id, data.email), role: data.role };
      expect(created.id > cursor).toBe(true);
      return route.fulfill({ json: created });
    }
    const after = new URL(request.url()).searchParams.get('after');
    cursors.push(after);
    if (!after) return route.fulfill({ json: { items: first, next: cursor } });
    expect(after).toBe(cursor);
    expect(created).not.toBeNull();
    const second = [created, last].sort((a, b) => a.id.localeCompare(b.id));
    return route.fulfill({ json: { items: second, next: null } });
  });
  await enterDirectory(page);
  const panel = region(page);
  await expect(panel.getByRole('listitem')).toHaveCount(20);
  await panel
    .getByLabel('Correo de la invitacion', { exact: true })
    .fill('new-member@example.test');
  await panel.getByRole('button', { name: 'Enviar invitacion', exact: true }).click();
  await expect(panel.getByRole('listitem')).toHaveCount(21);
  await panel.getByRole('button', { name: 'Cargar mas invitaciones', exact: true }).click();
  await expect(panel).toHaveAttribute('aria-busy', 'false');
  expect(pageErrors).toEqual([]);
  await expect(panel.getByRole('listitem')).toHaveCount(22);
  for (const row of [...first, created, last]) {
    await expect(panel.getByText(row.email, { exact: true })).toHaveCount(1);
  }
  expect(cursors).toEqual([null, cursor]);
  await expect(
    panel.getByRole('button', { name: 'Cargar mas invitaciones', exact: true }),
  ).toHaveCount(0);
});
