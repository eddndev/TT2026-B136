import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  editor,
  fillDeadline,
  confirmDeadline,
  deadlineError,
} from './deadline-editor-helpers.mjs';
import { id } from '../fixtures/deadline-unit.mjs';
import {
  factRecord,
  factPrepared,
  factCommand,
  factResolutionSource,
  emptyFactSources,
  resolutionId,
  notificationId,
} from '../fixtures/procedural-facts.mjs';

test('responsible selector pages eligible staff without requiring a manually entered identifier', async ({
  page,
}) => {
  const state = await setupDeadlines(page);
  state.responsibles.push(
    ...Array.from({ length: 20 }, (_, n) => ({
      id: id(40 + n),
      email: `responsible-${n}@example.test`,
      role: 'paralegal',
    })),
  );
  await openDeadlines(page);
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fillDeadline(page);
  const form = editor(page);
  await form
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('follow');
  await form.getByRole('button', { name: 'Elegir responsable', exact: true }).click();
  await form.getByRole('button', { name: 'Siguientes responsables', exact: true }).click();
  await form
    .getByRole('button', { name: 'Elegir responsable responsible-19@example.test', exact: true })
    .click();
  await confirmDeadline(page);
  await expect(form).toHaveCount(0);
  expect(state.submissions[0].change.definition.responsible_id).toBe(id(59));
  expect(state.calls.filter((row) => row.path.endsWith('/responsibles')).at(-1).search).toContain(
    `after_id=${id(58)}`,
  );
});

test('notification selector keeps the exact historical parent resolution revision', async ({
  page,
}) => {
  const original = factRecord(),
    parentCommand = factCommand('resolution', 'correct', 1);
  parentCommand.change.values.summary = 'Resolucion con una revision mas reciente';
  const newer = factRecord(factPrepared(parentCommand));
  const notification = factRecord(
    factPrepared(factCommand('notification'), null, {
      ...emptyFactSources(),
      resolution: factResolutionSource(original),
    }),
  );
  const state = await setupDeadlines(page, { facts: [original, newer, notification] });
  state.handle = async (route, call) => {
    if (call.path.endsWith('/prepare')) {
      await deadlineError(route, 'invalid_deadline', 422);
      return true;
    }
  };
  await openDeadlines(page);
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fillDeadline(page);
  const form = editor(page);
  await form
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('follow');
  await form
    .getByRole('combobox', { name: 'Tipo de fuente', exact: true })
    .selectOption('notification');
  await form.getByRole('button', { name: 'Elegir fuente exacta', exact: true }).click();
  await form
    .getByRole('button', {
      name: `Consultar notificaciones de resolucion ${resolutionId}`,
      exact: true,
    })
    .click();
  await form
    .getByRole('button', { name: `Revisiones de notificaci\u00f3n ${notificationId}`, exact: true })
    .click();
  await form
    .getByRole('button', { name: 'Consultar notificaci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await form
    .getByRole('button', { name: 'Vincular esta notificaci\u00f3n exacta', exact: true })
    .click();
  await expect(
    form.getByText('Resoluci\u00f3n vinculada: revisi\u00f3n 1', { exact: true }),
  ).toBeVisible();
  await form
    .getByRole('combobox', { name: 'Cuando cambie la fuente', exact: true })
    .selectOption('fixed');
  await form.getByRole('button', { name: 'Preparar plazo', exact: true }).click();
  await expect(form.getByRole('alert')).toBeVisible();
  expect(
    state.calls.find((row) => row.path.endsWith('/prepare')).body.change.definition.input.selection
      .source,
  ).toEqual({
    kind: 'known',
    value: {
      family: 'notification',
      id: notificationId,
      revision: 1,
      resolution: { id: resolutionId, revision: 1 },
    },
  });
});
