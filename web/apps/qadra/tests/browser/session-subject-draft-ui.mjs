import { expect } from '@playwright/test';
import { document } from './helpers.mjs';
import { openParticipant, detail } from './participant-helpers.mjs';
import { openParticipants } from './session-participant-drafts-helpers.mjs';
import {
  typed,
  subjectPath,
  reviewPath,
  casePath,
  candidate,
  candidateId,
  subject,
} from './session-subject-draft-fixtures.mjs';

export const raw = {
  name: '  Nombre en revision  ',
  curp: 'abc-12',
  curpReason: '  Dato pendiente de confirmar  ',
  locator: '  Pagina sin terminar  ',
  reason: '  Motivo escrito sin confirmar  ',
  different: '  Homonimo por contrastar  ',
  search: '  soporte por buscar  ',
};
export const subjectModal = (page) =>
  page.getByRole('dialog', {
    name: 'Editar identidad del expediente',
    exact: true,
  });
export const saveSubject = (modal) =>
  modal.getByRole('button', {
    name: 'Guardar revisi\u00f3n de identidad',
    exact: true,
  });
export const identitySupport = (modal) =>
  modal.getByRole('group', {
    name: 'Soporte de identidad',
    exact: true,
  });
export const currentIdentity = (page) =>
  detail(page).getByRole('region', {
    name: 'Identidad actual consultada',
    exact: true,
  });
export const candidateRegion = (modal) =>
  modal.getByRole('region', {
    name: 'Candidato Otra identidad',
    exact: true,
  });

export async function readIdentity(page) {
  await openParticipants(page);
  await openParticipant(page, typed.display_name);
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await expect(currentIdentity(page)).toBeVisible();
}

export async function editReadIdentity(page) {
  await currentIdentity(page)
    .getByRole('button', { name: 'Editar identidad', exact: true })
    .click();
  const modal = subjectModal(page);
  await expect(modal).toBeVisible();
  return modal;
}

export async function openIdentity(page) {
  await readIdentity(page);
  return editReadIdentity(page);
}

export async function fillIdentity(modal, invalid = false) {
  await modal.getByLabel('Nombre de la persona', { exact: true }).fill(raw.name);
  await modal
    .getByLabel('Estado de CURP', { exact: true })
    .selectOption(invalid ? 'known' : 'unknown');
  await modal
    .getByLabel(invalid ? 'CURP' : 'Motivo de CURP', { exact: true })
    .fill(invalid ? raw.curp : raw.curpReason);
  await identitySupport(modal)
    .getByLabel('P\u00e1gina o secci\u00f3n', { exact: true })
    .fill(raw.locator);
  return modal.elementHandle();
}

export async function expectIdentity(modal, invalid = false) {
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(raw.name);
  await expect(modal.getByLabel('Estado de CURP', { exact: true })).toHaveValue(
    invalid ? 'known' : 'unknown',
  );
  await expect(modal.getByLabel(invalid ? 'CURP' : 'Motivo de CURP', { exact: true })).toHaveValue(
    invalid ? raw.curp : raw.curpReason,
  );
  await expect(
    identitySupport(modal).getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }),
  ).toHaveValue(raw.locator);
}

export async function reviewIdentity(page, modal, state) {
  state.reviewBudget++;
  const response = page.waitForResponse(
    (result) =>
      new URL(result.url()).pathname === reviewPath && result.request().method() === 'POST',
  );
  await modal
    .getByRole('button', { name: 'Revisar coincidencias de identidad', exact: true })
    .click();
  expect((await response).status()).toBe(200);
  await expect(
    modal.getByRole('heading', { name: 'Revisi\u00f3n de identidad', exact: true }),
  ).toBeVisible();
  return state.reviews.at(-1);
}

export async function prepareIdentity(page, modal, state) {
  await reviewIdentity(page, modal, state);
  await modal.getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true }).fill(raw.reason);
}

export function setCandidate(state, revision = 1) {
  state.candidates = [{ ...candidate, reference: { ...candidate.reference, revision } }];
  state.subjects.set(candidateId, [
    {
      ...subject,
      id: candidateId,
      revision,
      values: { ...subject.values, name: { state: 'known', value: 'Otra identidad' } },
    },
  ]);
  state.directoryStamp = String(revision).repeat(64);
}

export async function declareDifferent(modal) {
  const region = candidateRegion(modal);
  await region.getByRole('button', { name: 'Declarar persona distinta', exact: true }).click();
  await region.getByLabel('Motivo de candidato distinto', { exact: true }).fill(raw.different);
  return region.getByRole('group', { name: 'Soporte de comparaci\u00f3n', exact: true });
}

export async function pickExactSupport(group) {
  await group.getByRole('button', { name: 'Seleccionar documento', exact: true }).click();
  await group
    .getByRole('button', { name: `${document.name} / versi\u00f3n actual 1`, exact: true })
    .click();
  await group
    .getByRole('button', {
      name: `Versi\u00f3n 1 / ${document.name} / Actual al consultar`,
      exact: true,
    })
    .click();
  await group.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await group.getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }).fill(raw.locator);
}

export async function openChildUpload(page, group) {
  await group.getByRole('button', { name: 'Cargar soporte', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await expect(modal).toBeVisible();
  return modal;
}

export async function fillChild(modal, name, text) {
  await modal
    .getByLabel('Archivo', { exact: true })
    .setInputFiles({ name, mimeType: 'text/plain', buffer: Buffer.from(text) });
  await modal
    .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
    .fill('  Soporte en revision  ');
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill('  etiqueta parcial  ');
  return modal.elementHandle();
}

export function expectFreshSubject(state, before) {
  const calls = state.calls.slice(before);
  const caseIndex = calls.findIndex((call) => call.path === casePath && call.method === 'GET');
  const identityIndex = calls.findIndex(
    (call, index) => index > caseIndex && call.path === subjectPath && call.method === 'GET',
  );
  expect(caseIndex).toBeGreaterThanOrEqual(0);
  expect(identityIndex).toBeGreaterThan(caseIndex);
  for (const call of [calls[caseIndex], calls[identityIndex]])
    expect(call).toMatchObject({
      body: null,
      search: '',
      headers: { authorization: `Bearer ${state.current.token}` },
    });
}
