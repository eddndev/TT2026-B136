import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { openParticipants } from './session-participant-drafts-helpers.mjs';
import {
  typedDraftSetup,
  checkTypedRequests,
  subjectsPath,
} from './session-typed-draft-fixtures.mjs';
import {
  setCandidate,
  candidateRegion,
  declareDifferent,
  pickExactSupport,
  raw,
} from './session-subject-draft-ui.mjs';
import {
  createTyped,
  beginTyped,
  fillTypedOwn,
  expectTypedOwn,
  rawTyped,
  typedModal,
  reviewTyped,
  confirmTyped,
} from './session-typed-draft-ui.mjs';

test.afterEach(async ({ page }) => checkTypedRequests(page));

test('typed identity search retains unapplied input and a different candidate revision never inherits the saved decision', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  setCandidate(state);
  state.typedCandidates = state.candidates;
  await login(page);
  let modal = await createTyped(page);
  await fillTypedOwn(modal);
  await modal.getByRole('button', { name: 'Elegir identidad existente', exact: true }).click();
  let picker = modal.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await picker
    .getByLabel('Buscar identidad por nombre', { exact: true })
    .fill('  Nombre aun sin buscar  ');
  await picker
    .getByRole('combobox', { name: 'Tipo de identidad a buscar', exact: true })
    .selectOption('natural_person');
  await expire(page, state, await modal.elementHandle());
  await login(page);
  modal = await createTyped(page);
  await expectTypedOwn(modal);
  const before = state.calls.length;
  await modal.getByRole('button', { name: 'Elegir identidad existente', exact: true }).click();
  picker = modal.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await expect(picker.getByLabel('Buscar identidad por nombre', { exact: true })).toHaveValue(
    '  Nombre aun sin buscar  ',
  );
  await expect(
    picker.getByRole('combobox', { name: 'Tipo de identidad a buscar', exact: true }),
  ).toHaveValue('natural_person');
  await expect(
    picker.getByRole('button', { name: 'Consultar identidad: Persona tipificada', exact: true }),
  ).toBeVisible();
  const fresh = state.calls.slice(before).filter((call) => call.path === subjectsPath);
  expect(fresh.length).toBeGreaterThan(0);
  for (const call of fresh) {
    expect(new URLSearchParams(call.search).has('name')).toBe(false);
    expect(new URLSearchParams(call.search).has('kind')).toBe(false);
  }
  await picker.getByRole('button', { name: 'Cerrar selector de identidad', exact: true }).click();
  await reviewTyped(page, modal, state);
  await modal
    .getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true })
    .fill(rawTyped.reason);
  await pickExactSupport(await declareDifferent(modal));
  await expire(page, state, await modal.elementHandle());
  setCandidate(state, 2);
  state.typedCandidates = state.candidates;
  await login(page);
  modal = await createTyped(page);
  await expectTypedOwn(modal);
  expect(state.typedReviews).toHaveLength(1);
  await reviewTyped(page, modal, state);
  const current = candidateRegion(modal);
  await expect(current).toContainText('revisi\u00f3n 2');
  await expect(current.getByLabel('Motivo de candidato distinto', { exact: true })).toHaveCount(0);
  await expect
    .poll(() =>
      modal.locator('textarea:visible').evaluateAll((rows) => rows.map((row) => row.value)),
    )
    .toContain(raw.different);
  await current.getByRole('button', { name: 'Declarar persona distinta', exact: true }).click();
  await expect(current.getByLabel('Motivo de candidato distinto', { exact: true })).toHaveValue('');
  await modal.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await expect(modal.getByRole('alert')).toContainText('Motivo de candidato distinto');
  expect(state.typedPreparations).toEqual([]);
  expect(state.typedCommits).toEqual([]);
});

test('closed typed drafts stay readonly and denial, account changes and cancellation discard their values', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  await login(page);
  let modal = await createTyped(page);
  await fillTypedOwn(modal);
  await expire(page, state, await modal.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  await login(page);
  await openParticipants(page);
  await expect(
    page.getByRole('button', { name: 'Agregar participante', exact: true }),
  ).toBeDisabled();
  await page.getByRole('button', { name: 'Retomar borrador tipificado', exact: true }).click();
  modal = typedModal(page);
  await expectTypedOwn(modal);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toBeDisabled();
  await expect(confirmTyped(modal)).toBeDisabled();
  await expire(page, state, await modal.elementHandle());
  state.caseStatus = 'active';
  state.caseRevision++;
  await login(page);
  await openParticipants(page);
  state.allowed = false;
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(typedModal(page)).toBeHidden();
  await expect
    .poll(() => page.locator('input,textarea').evaluateAll((rows) => rows.map((row) => row.value)))
    .not.toContain(rawTyped.name);
  state.allowed = true;
  await page
    .getByRole('region', { name: 'Directorio del expediente', exact: true })
    .getByRole('button', { name: 'Actualizar', exact: true })
    .click();
  modal = await beginTyped(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue('');
  await fillTypedOwn(modal);
  await expire(page, state, await modal.elementHandle());
  await signInOther(page);
  await selectCase(page);
  modal = await createTyped(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue('');
  await fillTypedOwn(modal);
  const element = await modal.elementHandle();
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expire(page, state, element);
  await signInOther(page);
  await selectCase(page);
  modal = await createTyped(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue('');
  expect(state.typedCommits).toEqual([]);
  expect(state.uploads).toEqual([]);
});
