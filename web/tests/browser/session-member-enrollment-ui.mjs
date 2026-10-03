import { expect } from '@playwright/test';
import { navigate } from './helpers.mjs';
import { directory } from './members-helpers.mjs';
import {
  initialPassword,
  enrollmentSecret,
  enrollmentCode,
} from './session-member-enrollment-fixtures.mjs';

export { directory };
export const rawEmail = 'Pendiente.Alta@';
export const panel = (page) => page.locator('.admin-panel');
export const email = (page) => panel(page).getByLabel('Correo del nuevo usuario', { exact: true });
export const password = (page) =>
  panel(page).getByLabel('Contrase\u00f1a inicial', { exact: true });
export const role = (page) => panel(page).getByRole('combobox', { name: 'Rol', exact: true });
export const create = (page) =>
  panel(page).getByRole('button', { name: 'Crear usuario', exact: true });
export const resume = (page) =>
  page.getByRole('button', { name: 'Retomar alta de integrante', exact: true });
export const discard = (page) =>
  page.getByRole('button', { name: 'Descartar alta y volver al directorio', exact: true });
export const consult = (page) =>
  panel(page).getByRole('button', { name: 'Consultar directorio para esta cuenta', exact: true });

export async function enterEnrollment(page) {
  await navigate(page, 'Equipo');
  await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
}

export async function fillEnrollment(page, address = rawEmail) {
  await email(page).fill(address);
  await role(page).selectOption('litigator');
  await password(page).fill(initialPassword);
  return panel(page).elementHandle();
}

export async function expectNoEnrollmentMaterial(page) {
  await expect(
    page.getByRole('heading', { name: 'Configura el segundo factor', exact: true }),
  ).toHaveCount(0);
  await expect(page.getByText(enrollmentSecret, { exact: true })).toHaveCount(0);
  await expect(page.getByText(enrollmentCode, { exact: true })).toHaveCount(0);
  await expect(password(page)).toHaveValue('');
}

export async function expectBlankEnrollment(page) {
  await expect(resume(page)).toHaveCount(0);
  await expect(email(page)).toHaveValue('');
  await expect(role(page)).toHaveValue('paralegal');
  await expectNoEnrollmentMaterial(page);
}
