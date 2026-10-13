import { test, expect } from '@playwright/test';
import {
  setupDerivedDeadline,
  openDerivedDeadline,
  fillDerivedDeadline,
  derivedEditor,
} from './hearing-derived-deadline-helpers.mjs';

// A declared start with a different purpose remains blocked, not silently converted.
function orderedMismatchReview(state) {
  const profile = state.deadlines.profiles[0];
  profile.definition.template = {
    kind: 'ordered',
    unit: { kind: 'elapsed_hours' },
    maximum: 72,
  };
  profile.definition.examples[0].ordered_quantity = 24;
  const prepare = state.prepare;
  state.prepare = (command) => {
    const ready = prepare(command);
    ready.deadline.profile = structuredClone(profile);
    ready.deadline.profile_head = structuredClone(profile);
    const block = {
      kind: 'qualification_mismatch',
      expected: 'hearing_end',
      actual: 'ordered_period_start',
    };
    ready.deadline.result = {
      requirement: structuredClone(profile.definition.trigger),
      trigger_outcome: { kind: 'blocked', block },
      rule: { kind: 'elapsed_hours', quantity: 48 },
      arithmetic: null,
      due_at: null,
      blocks: [
        { kind: 'condition_rejected', id: profile.definition.conditions[0].id },
        { kind: 'trigger', block: structuredClone(block) },
      ],
    };
    return ready;
  };
}

test('joint review exposes the selected agreement, declared start, quantity and applicability decisions', async ({
  page,
}) => {
  const state = await setupDerivedDeadline(page);
  orderedMismatchReview(state);
  await openDerivedDeadline(page);
  await fillDerivedDeadline(page);
  const form = derivedEditor(page);
  await form.getByText('Acuerdos declarados (0/16)', { exact: true }).click();
  await form.getByRole('button', { name: 'Agregar acuerdo declarado', exact: true }).click();
  await form
    .getByLabel('Texto del acuerdo 1', { exact: true })
    .fill('Acuerdo con respuesta escrita');
  await form
    .getByRole('combobox', { name: 'Acuerdo del resultado propuesto', exact: true })
    .selectOption({ label: 'Acuerdo 1: Acuerdo con respuesta escrita' });
  await form.getByRole('checkbox', { name: 'Declarar un inicio calificado', exact: true }).check();
  await form
    .getByRole('combobox', { name: 'Finalidad del inicio', exact: true })
    .selectOption('ordered_period_start');
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de inicio calificado', exact: true })
    .selectOption('second');
  await form.getByLabel('Fecha de inicio calificado', { exact: true }).fill('2026-09-02');
  await form.getByLabel('Hora de inicio calificado', { exact: true }).fill('13:14:15');
  await form
    .getByRole('combobox', { name: 'Desfase de inicio calificado', exact: true })
    .selectOption('declared');
  await form.getByLabel('Desfase UTC de inicio calificado', { exact: true }).fill('-06:00');
  await form
    .getByLabel('Declaraci\u00f3n del inicio', { exact: true })
    .fill('Inicio comunicado del periodo concedido');
  await form.getByLabel('Localizador del inicio', { exact: true }).fill('Acuerdo, inciso A');
  await form
    .getByRole('combobox', { name: 'Cantidad ordenada', exact: true })
    .selectOption('known');
  await form.getByLabel('Cantidad declarada', { exact: true }).fill('48');
  await form
    .getByRole('combobox', { name: 'Se cumple la condicion 1', exact: true })
    .selectOption('no');
  await form
    .getByLabel('Localizador de condici\u00f3n 1', { exact: true })
    .fill('Acuerdo, inciso B');
  await form.getByRole('button', { name: 'Preparar resultado y plazo', exact: true }).click();
  const review = form.locator('[aria-label="Revision conjunta"]');
  await expect(review).toBeVisible();
  await expect(review).toContainText('Acuerdo seleccionado: Acuerdo con respuesta escrita');
  await expect(review).toContainText('Inicio declarado del periodo concedido');
  await expect(review).toContainText('2026-09-02 / 13:14:15 / UTC-06:00');
  await expect(review).toContainText('Inicio comunicado del periodo concedido');
  await expect(review).toContainText('Acuerdo, inciso A');
  await expect(review).toContainText('Cantidad ordenada: 48');
  await expect(review).toContainText(/El [a\u00e1]mbito aplica:\s*S[i\u00ed]/);
  await expect(review).toContainText(/Existe incidencia sin resolver:\s*No/);
  await expect(review).toContainText(
    /Condici[o\u00f3]n 1:\s*Supuesto declarado\s*\/\s*No\s*\/\s*Acuerdo, inciso B/,
  );
  await expect(review).toContainText('Sin vencimiento calculado');
  expect(state.submissions).toHaveLength(0);
  expect(state.calls).toHaveLength(1);
  const command = state.calls[0].body;
  expect(command.deadline.change.definition.input.selection.source.value.agreement_id).toBe(
    command.result.change.values.agreements[0].id,
  );
  expect(command.deadline.change.definition.input.ordered_quantity).toBe(48);
});
