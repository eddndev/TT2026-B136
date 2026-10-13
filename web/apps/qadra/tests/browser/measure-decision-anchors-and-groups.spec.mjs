import { test, expect } from '@playwright/test';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { seedDecisionGroup } from '../fixtures/measure-decision-group-browser.mjs';
import {
  setupMeasureDecisions,
  openMeasures,
  newDecision,
  fillDecisionCommon,
  prepareDecision,
  submitDecision,
  decisionEditor,
  decisionDetail,
  clone,
} from './measure-decision-helpers.mjs';

function existingInitialAnchor(state) {
  const hearing = hearingRecord();
  const captured = {
    case_id: state.caseId,
    id: '11111111-1111-4111-8111-111111111111',
    revision: 1,
    display_name: 'Defensa capturada en audiencia inicial',
    procedural_role: 'Defensa',
    organization: 'Despacho de origen',
    directory_status: 'active',
    kind: null,
    canonical_format: 'part1',
    subject: null,
  };
  const valuesDigest = 'e'.repeat(64);
  hearing.values.participants = [{ participant_id: captured.id, revision: captured.revision }];
  hearing.participants = [
    {
      id: captured.id,
      revision: captured.revision,
      profile: 'manual',
      display_name: captured.display_name,
      procedural_role: captured.procedural_role,
      kind: null,
      subject: null,
      values_digest: valuesDigest,
      directory_status: 'active',
    },
  ];
  hearing.recorded_by = { id: state.actor.id, email: state.actor.email };
  hearing.scheduling_context.administration_digest = state.context.administration.values_digest;
  hearing.recorded_administration_digest = state.context.administration.values_digest;
  state.scheduling.hearings.state.records.set(hearing.id, [clone(hearing)]);
  Object.assign(state.scheduling.hearings.state.participants[0], {
    revision: 2,
    display_name: 'Nombre posterior ajeno a la captura inicial',
  });
  state.expectedAnchor = {
    kind: 'initial',
    hearing_id: hearing.id,
    revision: hearing.revision,
    values_digest: hearing.values_digest,
    submission_digest: hearing.receipt.submission_digest,
  };
  state.anchorMaterial = {
    kind: 'initial',
    hearing: {
      ...clone(hearing),
      participants: [{ overview: captured, values_digest: valuesDigest }],
    },
  };
  return { hearing, captured, valuesDigest };
}

test('an existing initial hearing anchors the decision and exposes captured participants without creating another hearing', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page);
  const { hearing, captured, valuesDigest } = existingInitialAnchor(state);
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  const editor = decisionEditor(page);
  await editor
    .getByRole('combobox', { name: 'Resultado', exact: true })
    .selectOption('no_measure_change');
  await editor
    .getByLabel(/Declaraci[o\u00f3]n sin cambios/, { exact: true })
    .fill('Decision vinculada a la audiencia inicial existente');
  await editor
    .getByRole('combobox', { name: 'Vinculo de audiencia', exact: true })
    .selectOption('initial');
  const picker = editor.getByRole('region', {
    name: /Elegir programaci[o\u00f3]n hist[o\u00f3]rica/,
  });
  await picker
    .getByRole('button', { name: new RegExp(`Consultar programaci[o\\u00f3]n ${hearing.id}`) })
    .click();
  await picker
    .getByRole('button', { name: /Usar programaci[o\u00f3]n revisi[o\u00f3]n 1/ })
    .click();
  await prepareDecision(page);
  const anchor = editor.getByRole('region', {
    name: 'Audiencia de origen de la decision',
    exact: true,
  });
  await expect(anchor.getByText(captured.display_name, { exact: true })).toBeVisible();
  await expect(anchor).toContainText(captured.organization);
  await expect(anchor).not.toContainText('Nombre posterior ajeno a la captura inicial');
  await anchor.getByText('Referencia exacta del participante', { exact: true }).click();
  await expect(anchor.getByText(valuesDigest, { exact: true })).toBeVisible();
  expect(state.preparations[0].review.material.anchor.hearing.participants).toEqual([
    { overview: captured, values_digest: valuesDigest },
  ]);
  await submitDecision(page);
  await expect(decisionDetail(page)).toContainText(captured.display_name);
  expect(state.submissions[0].command.anchor).toEqual(state.expectedAnchor);
  expect(state.scheduling.hearings.state.records.size).toBe(1);
  expect(state.scheduling.hearings.state.records.get(hearing.id)).toHaveLength(1);
  expect(state.scheduling.hearings.state.calls.filter((call) => call.method !== 'GET')).toEqual([]);
  expect(state.scheduling.hearings.state.submissions).toEqual([]);
  expect(state.scheduling.records.size).toBe(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.unexpected).toEqual([]);
});

async function selectEffect(page, index, action, references) {
  const effect = decisionEditor(page).getByRole('group', { name: `Efecto ${index}`, exact: true });
  await effect
    .getByRole('combobox', { name: `Accion de medida ${index}`, exact: true })
    .selectOption(action);
  for (const reference of references) {
    await effect.getByRole('button', { name: 'Elegir medida', exact: true }).click();
    const picker = effect.getByRole('region', { name: 'Seleccionar medida exacta', exact: true });
    await picker
      .getByRole('button', { name: `Consultar medida ${reference.id}`, exact: true })
      .click();
    await expect(picker).toContainText(reference.capture_digest);
    await picker.getByRole('button', { name: 'Vincular esta revision', exact: true }).click();
    await expect(picker).toHaveCount(0);
  }
  return effect;
}

async function fillSuccessor(effect, index, kind, conditions) {
  await effect.getByRole('button', { name: 'Agregar medida sustituta', exact: true }).click();
  const fields = effect.getByRole('group', { name: `Medida sustituta ${index}`, exact: true });
  await fields.getByRole('button', { name: 'Elegir sujeto', exact: true }).click();
  const picker = fields.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await picker
    .getByRole('button', { name: 'Consultar identidad: Persona declarada', exact: true })
    .click();
  await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  await fields.getByRole('combobox', { name: 'Clase de medida', exact: true }).selectOption(kind);
  await fields.getByLabel('Condiciones', { exact: true }).fill(conditions);
  await fields
    .getByRole('combobox', { name: /Precisi[o\u00f3]n de inicio de vigencia/, exact: true })
    .selectOption('unknown');
  await fields
    .getByLabel('Motivo de tiempo desconocido de inicio de vigencia', { exact: true })
    .fill('Inicio no declarado en el soporte');
  await fields
    .getByLabel(/Declaraci[o\u00f3]n de vigencia/, { exact: true })
    .fill('Vigencia declarada para la sustituta');
  await fields
    .getByRole('combobox', { name: /Supervisi[o\u00f3]n/, exact: true })
    .selectOption('unknown');
  await fields
    .getByLabel(/Motivo de supervisi[o\u00f3]n desconocida/, { exact: true })
    .fill('No consta supervisor de la sustituta');
}

test('one reviewed group confirms ceases and substitutes two prior measures with two declared successors', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page);
  const original = seedDecisionGroup(state);
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  const editor = decisionEditor(page);
  await selectEffect(page, 1, 'confirm', [original[0].reference]);
  await editor.getByRole('button', { name: 'Agregar efecto', exact: true }).click();
  await selectEffect(page, 2, 'cease', [original[1].reference]);
  await editor.getByRole('button', { name: 'Agregar efecto', exact: true }).click();
  const substitution = await selectEffect(
    page,
    3,
    'substitute',
    original.slice(2).map((row) => row.reference),
  );
  await fillSuccessor(
    substitution,
    1,
    'periodic_appearance',
    'Comparecer conforme a sustitucion declarada',
  );
  await fillSuccessor(
    substitution,
    2,
    'travel_restriction',
    'Permanecer en el ambito declarado en soporte',
  );
  await prepareDecision(page);
  const review = editor.getByRole('region', { name: 'Revision de decision cautelar', exact: true });
  await expect(review.getByRole('heading', { name: /^Confirmaci[o\u00f3]n \// })).toBeVisible();
  await expect(review.getByRole('heading', { name: /^Cese \// })).toBeVisible();
  await expect(review.getByRole('heading', { name: /^Sustituida \// })).toHaveCount(2);
  await expect(review.getByRole('heading', { name: /^Sustituta \// })).toHaveCount(2);
  await expect(review).toContainText('Comparecer conforme a sustitucion declarada');
  await expect(review).toContainText('Permanecer en el ambito declarado en soporte');
  await submitDecision(page);
  await expect(decisionDetail(page)).toContainText('Comparecer conforme a sustitucion declarada');
  const sent = state.submissions[0],
    effects = sent.command.outcome.effects;
  expect(effects).toHaveLength(3);
  expect(effects.find((row) => row.action === 'confirm')).toEqual({
    action: 'confirm',
    previous: original[0].reference,
  });
  expect(effects.find((row) => row.action === 'cease')).toEqual({
    action: 'cease',
    previous: original[1].reference,
  });
  const replacement = effects.find((row) => row.action === 'substitute');
  expect(replacement.predecessors).toEqual(original.slice(2).map((row) => row.reference));
  expect(replacement.successors).toHaveLength(2);
  for (const next of replacement.successors)
    expect(next.values.subject).toEqual({
      id: state.subject.id,
      revision: state.subject.revision,
      values_digest: state.subject.values_digest,
    });
  const receipt = state.operations.get(sent.command.operation_id),
    captures = receipt.group.measures;
  expect(captures).toHaveLength(6);
  expect(new Set(captures.map((row) => row.operation_id))).toEqual(
    new Set([sent.command.operation_id]),
  );
  for (const previous of original) {
    const current = state.records.get(previous.reference.id).at(-1);
    expect(current.reference.revision).toBe(2);
    expect(current.judicial_origin).toEqual(previous.judicial_origin);
  }
  expect(receipt.group.substitutions).toHaveLength(1);
  expect(receipt.group.substitutions[0].predecessors.map((row) => row.previous)).toEqual(
    replacement.predecessors,
  );
  expect(receipt.group.substitutions[0].successors.map((row) => row.id)).toEqual(
    replacement.successors.map((row) => row.id),
  );
  expect(receipt.record_history.records.judicial.groups).toEqual(
    original[0].record_history.records.judicial.groups,
  );
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toHaveLength(1);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.unexpected).toEqual([]);
});
