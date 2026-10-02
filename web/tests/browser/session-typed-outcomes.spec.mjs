import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { detail, openParticipant, participant } from './participant-helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { openParticipants } from './session-participant-drafts-helpers.mjs';
import {
  typedDraftSetup,
  checkTypedRequests,
  holdTypedRequest,
  typed,
  participantsPath,
  proposalPath,
} from './session-typed-draft-fixtures.mjs';
import { createdParticipantId } from './session-typed-command-fixtures.mjs';
import {
  completeTyped,
  createTyped,
  fillTypedOwn,
  expectTypedOwn,
  typedModal,
  confirmTyped,
  prepareTypedFresh,
} from './session-typed-draft-ui.mjs';

test.afterEach(async ({ page }) => checkTypedRequests(page));

test('a manual completion draft stays distinct and readonly when another editor has already typed that record', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  await login(page);
  let modal = await completeTyped(page);
  await fillTypedOwn(modal);
  await expire(page, state, await modal.elementHandle());
  const advanced = {
    ...typed,
    id: participant.id,
    revision: 2,
    display_name: 'Ficha completada por otro editor',
    organization: 'Oficina vigente',
  };
  state.records.get(participant.id).push(advanced);
  await login(page);
  await openParticipants(page);
  await openParticipant(page, advanced.display_name);
  await detail(page)
    .getByRole('button', { name: 'Retomar borrador tipificado', exact: true })
    .click();
  modal = typedModal(page, 'Completar perfil de participante');
  await expectTypedOwn(modal);
  await expect(modal.getByLabel('Organizaci\u00f3n (opcional)', { exact: true })).toBeDisabled();
  await expect(confirmTyped(modal)).toBeDisabled();
  await expect(modal).toContainText('tipificada');
  expect(state.typedReviews).toEqual([]);
  expect(state.typedCommits).toEqual([]);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await detail(page).getByRole('button', { name: 'Editar participante', exact: true }).click();
  modal = typedModal(page, 'Editar ficha tipificada');
  await expect(modal.getByLabel('Organizaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    'Oficina vigente',
  );
  expect(state.records.get(participant.id)[0]).toEqual(participant);
});

test('an interrupted typed commit reconciles its exact submission without automatic resend or revival after confirmation', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  await login(page);
  let modal = await createTyped(page);
  await fillTypedOwn(modal);
  await prepareTypedFresh(page, modal, state);
  const gate = holdTypedRequest(state, 'POST', `${proposalPath}/commit`);
  const oldBearer = `Bearer ${state.current.token}`;
  const late = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `${proposalPath}/commit` &&
      response.request().headers().authorization === oldBearer,
  );
  state.nextTypedCommit = { status: 503 };
  await confirmTyped(modal).click();
  await expect.poll(() => gate.entered).toBe(true);
  const sent = structuredClone(state.typedCommits[0].values);
  await expire(page, state, await modal.elementHandle());
  await login(page);
  modal = await createTyped(page);
  await expectTypedOwn(modal);
  await expect(confirmTyped(modal)).toBeDisabled();
  expect(state.typedCommits).toHaveLength(1);
  expect(state.typedReviews).toHaveLength(1);
  expect(state.typedPreparations).toHaveLength(1);
  gate.release();
  expect((await late).status()).toBe(503);
  await expect(confirmTyped(modal)).toBeDisabled();
  const exactPath = `${participantsPath}/${createdParticipantId}/revisions/1`;
  const result = page.waitForResponse((response) => new URL(response.url()).pathname === exactPath);
  const element = await modal.elementHandle();
  await modal
    .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
    .click();
  expect((await result).status()).toBe(200);
  await expect(modal).toBeHidden();
  expect(state.typedCommits).toHaveLength(1);
  expect(state.typedCommits[0].values).toEqual(sent);
  await expire(page, state, element);
  await login(page);
  modal = await createTyped(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue('');
  await expect(
    modal.getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true }),
  ).toHaveCount(0);
  expect(state.typedCommits).toHaveLength(1);
});
