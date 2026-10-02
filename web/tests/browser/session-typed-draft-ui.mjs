import { expect } from '@playwright/test';
import { detail, openParticipant, participant } from './participant-helpers.mjs';
import { openParticipants } from './session-participant-drafts-helpers.mjs';
import { pickExactSupport } from './session-subject-draft-ui.mjs';
import { typed, proposalPath } from './session-typed-draft-fixtures.mjs';

export const rawTyped = {
  name: '  Persona por revisar  ',
  curp: 'parcial-9',
  curpReason: '  Dato aun no contrastado  ',
  custody: '  Situacion por confirmar  ',
  organization: '  Oficina sin confirmar  ',
  legal: '  Borrador juridico parcial  ',
  reason: '  Revision escrita por la persona  ',
};
export const typedModal = (page, title = 'Agregar participante tipificado') =>
  page.getByRole('dialog', { name: title, exact: true });
export const confirmTyped = (modal) =>
  modal.getByRole('button', {
    name: 'Confirmar registro',
    exact: true,
  });
export const roleSupport = (modal) =>
  modal.getByRole('group', {
    name: 'Soporte del rol',
    exact: true,
  });

export async function createTyped(page) {
  await openParticipants(page);
  return beginTyped(page);
}

export async function beginTyped(page) {
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  const modal = typedModal(page);
  await expect(modal).toBeVisible();
  return modal;
}

export async function editTyped(page) {
  await openParticipants(page);
  await openParticipant(page, typed.display_name);
  await detail(page).getByRole('button', { name: 'Editar participante', exact: true }).click();
  const modal = typedModal(page, 'Editar ficha tipificada');
  await expect(modal).toBeVisible();
  return modal;
}

export async function completeTyped(page) {
  await openParticipants(page);
  await openParticipant(page, participant.display_name);
  await detail(page).getByRole('button', { name: 'Completar perfil', exact: true }).click();
  const modal = typedModal(page, 'Completar perfil de participante');
  await expect(modal).toBeVisible();
  return modal;
}

export async function fillTypedOwn(modal, invalid = false) {
  await modal.getByLabel('Nombre de la persona', { exact: true }).fill(rawTyped.name);
  await modal
    .getByLabel('Estado de CURP', { exact: true })
    .selectOption(invalid ? 'known' : 'unknown');
  await modal
    .getByLabel(invalid ? 'CURP' : 'Motivo de CURP', { exact: true })
    .fill(invalid ? rawTyped.curp : rawTyped.curpReason);
  await pickExactSupport(modal.getByRole('group', { name: 'Soporte de identidad', exact: true }));
  await modal.getByLabel('Tipo de participante', { exact: true }).selectOption('defendant');
  await modal.getByLabel('Estado de Situaci\u00f3n de libertad declarada').selectOption('unknown');
  await modal.getByLabel('Motivo de Situaci\u00f3n de libertad declarada').fill(rawTyped.custody);
  await modal
    .getByLabel('Organizaci\u00f3n (opcional)', { exact: true })
    .fill(rawTyped.organization);
  await modal
    .getByLabel('Situaci\u00f3n jur\u00eddica declarada (opcional)', { exact: true })
    .fill(rawTyped.legal);
  await pickExactSupport(roleSupport(modal));
}

export async function expectTypedOwn(modal, invalid = false) {
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    rawTyped.name,
  );
  await expect(modal.getByLabel(invalid ? 'CURP' : 'Motivo de CURP', { exact: true })).toHaveValue(
    invalid ? rawTyped.curp : rawTyped.curpReason,
  );
  await expect(modal.getByLabel('Motivo de Situaci\u00f3n de libertad declarada')).toHaveValue(
    rawTyped.custody,
  );
  await expect(modal.getByLabel('Organizaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    rawTyped.organization,
  );
  await expect(
    modal.getByLabel('Situaci\u00f3n jur\u00eddica declarada (opcional)', { exact: true }),
  ).toHaveValue(rawTyped.legal);
}

export async function reviewTyped(page, modal, state) {
  state.typedReviewBudget++;
  const response = page.waitForResponse(
    (row) => new URL(row.url()).pathname === `${proposalPath}/review`,
  );
  await modal
    .getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true })
    .click();
  expect((await response).status()).toBe(200);
  await expect(
    modal.getByRole('heading', { name: 'Revisi\u00f3n de identidad', exact: true }),
  ).toBeVisible();
}

export async function prepareTypedFresh(page, modal, state) {
  await reviewTyped(page, modal, state);
  await modal
    .getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true })
    .fill(rawTyped.reason);
  state.typedPrepareBudget++;
  const response = page.waitForResponse(
    (row) => new URL(row.url()).pathname === `${proposalPath}/prepare`,
  );
  await modal.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  expect((await response).status()).toBe(200);
  await expect(modal).toContainText('Preparaci\u00f3n para la revisi\u00f3n');
}

export async function chooseExistingSubject(modal) {
  await modal.getByRole('button', { name: 'Elegir identidad existente', exact: true }).click();
  const picker = modal.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await picker
    .getByRole('button', { name: `Consultar identidad: ${typed.display_name}`, exact: true })
    .click();
  await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
}
