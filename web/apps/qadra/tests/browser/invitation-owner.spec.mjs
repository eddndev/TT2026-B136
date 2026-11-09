import { test, expect } from '@playwright/test';
import { setupMembers, enterDirectory } from './members-helpers.mjs';
import { navigate } from './helpers.mjs';
const region = (page) =>
  page.getByRole('region', { name: 'Invitaciones al despacho', exact: true });
const summary = (data) => ({
  id: data.operation_id,
  operation_id: data.operation_id,
  revision: '1',
  email: data.email,
  role: data.role,
  status: 'pending',
  delivery: 'pending',
  issued_at: '2026-10-11T00:00:00Z',
  expires_at: '2026-10-12T00:00:00Z',
  accepted_at: null,
});

test('Owner issues and explicitly resends without ever seeing recipient secrets', async ({
  page,
}) => {
  await setupMembers(page);
  const calls = [];
  let row = null;
  await page.route('**/api/v1/member-invitations**', async (route) => {
    const req = route.request(),
      path = new URL(req.url()).pathname;
    if (req.method() === 'GET')
      return route.fulfill({
        json: path.endsWith('/member-invitations') ? { items: row ? [row] : [], next: null } : row,
      });
    const data = req.postDataJSON();
    calls.push({ path, data });
    row = path.endsWith('/resend')
      ? { ...row, revision: '2', operation_id: data.operation_id }
      : summary(data);
    await route.fulfill({ json: row });
  });
  await enterDirectory(page);
  await region(page)
    .getByLabel('Nombre de la persona invitada', { exact: true })
    .fill(' Persona Invitada ');
  await region(page)
    .getByLabel('Correo de la invitaci\u00f3n', { exact: true })
    .fill('recipient@example.test');
  await region(page)
    .getByRole('combobox', { name: 'Rol de la invitaci\u00f3n', exact: true })
    .selectOption('litigator');
  await region(page).getByRole('button', { name: 'Enviar invitaci\u00f3n', exact: true }).click();
  await expect(region(page)).toContainText('recipient@example.test');
  await expect(region(page)).toContainText('Correo pendiente');
  await expect(region(page).locator('input[type="password"]')).toHaveCount(0);
  expect(calls).toHaveLength(1);
  expect(Object.keys(calls[0].data).sort()).toEqual(['email', 'name', 'operation_id', 'role']);
  expect(calls[0].data.name).toBe('Persona Invitada');
  await expect(region(page)).not.toContainText('Persona Invitada');
  await region(page)
    .getByRole('button', { name: 'Reenviar a recipient@example.test', exact: true })
    .click();
  await expect(region(page)).toContainText('El enlace anterior dejar\u00e1 de funcionar');
  await region(page).getByRole('button', { name: 'Confirmar reenv\u00edo', exact: true }).click();
  await expect(region(page).getByRole('status')).toContainText('Invitaci\u00f3n registrada');
  expect(calls).toHaveLength(2);
  expect(calls[1].data.expected_revision).toBe('1');
  expect(calls[1].data.operation_id).not.toBe(calls[0].data.operation_id);
});
