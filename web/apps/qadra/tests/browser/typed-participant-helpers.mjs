import { expect } from '@playwright/test';
import { participantSetup, participant, otherParticipantId } from './participant-helpers.mjs';
import { caseId, document } from './helpers.mjs';
export const subjectId = '44444444-4444-4444-8444-444444444444';
export const support = {
  document_id: document.id,
  version: 1,
  digest: document.digest,
  locator: 'Pagina 1',
};
export const subject = {
  case_id: caseId,
  id: subjectId,
  revision: 1,
  values_digest: 'b'.repeat(64),
  changed_at: participant.changed_at,
  changed_by: participant.changed_by,
  values: {
    kind: 'natural_person',
    name: { state: 'known', value: 'Persona tipificada' },
    curp: { state: 'unknown', reason: 'No consta en el soporte' },
    identity_support: support,
  },
};
export const typed = {
  ...participant,
  id: otherParticipantId,
  display_name: 'Persona tipificada',
  procedural_role: 'defendant',
  canonical_format: 'part2',
  profile: { kind: 'defendant', custody: { state: 'unknown', reason: 'Sin dato comprobado' } },
  subject,
  role_support: support,
  credential_origin: null,
  submission_digest: 'd'.repeat(64),
  submission_revision: 1,
};
export async function typedSetup(page, role = 'owner', initial = [participant]) {
  const state = await participantSetup(page, role, initial);
  state.subjects = new Map([[subjectId, [structuredClone(subject)]]]);
  state.candidates = [];
  await page.route('**/api/v1/cases/*/subjects**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    state.calls.push({
      path: url.pathname,
      method: request.method(),
      body: request.postDataJSON(),
      search: url.search,
    });
    const parts = url.pathname.split('/subjects')[1].split('/').filter(Boolean);
    if (!parts.length)
      return route.fulfill({
        json: {
          subjects: [...state.subjects.values()].map((rows) => {
            const row = rows.at(-1);
            return {
              case_id: caseId,
              id: row.id,
              revision: row.revision,
              kind: row.values.kind,
              display_name: row.values.name.value || row.values.name,
            };
          }),
          has_more: false,
          next_after_id: null,
        },
      });
    const rows = state.subjects.get(parts[0]);
    if (!rows) return route.fulfill({ status: 404 });
    if (parts[1] === 'history')
      return route.fulfill({
        json: { revisions: [...rows].reverse(), has_more: false, next_before_revision: null },
      });
    return route.fulfill({
      json: parts[1] === 'revisions' ? rows.find((row) => row.revision === +parts[2]) : rows.at(-1),
    });
  });
  await page.route('**/participants/proposals/*', async (route) => {
    const request = route.request(),
      body = request.postDataJSON(),
      path = new URL(request.url()).pathname;
    state.calls.push({ path, method: request.method(), body });
    if (path.endsWith('/review')) {
      const identity =
        body.subject.operation === 'keep'
          ? state.subjects.get(body.subject.reference.id).at(-1)
          : { ...subject, values: body.subject.values };
      state.proposedSubject = identity;
      const proposal = {
        subject:
          body.subject.operation === 'keep'
            ? body.subject
            : {
                operation: 'append',
                id: identity.id,
                expected_revision: 0,
                values: identity.values,
              },
        participant_id: body.participant.id || otherParticipantId,
        expected_participant_revision: body.participant.expected_revision || 0,
        values: {
          subject: {
            id: identity.id,
            revision: identity.revision,
            values_digest: identity.values_digest,
          },
          directory_status: 'active',
          role: body.role,
        },
      };
      return route.fulfill({
        json: {
          case_id: caseId,
          proposal,
          directory_stamp: 'c'.repeat(64),
          candidates: state.candidates,
        },
      });
    }
    if (path.endsWith('/prepare')) {
      let declaration = null;
      if (body.certificate_base64) {
        const bytes = Buffer.alloc(218);
        bytes.write('PCRED1');
        declaration = {
          bytes_base64: bytes.toString('base64'),
          digest: 'e'.repeat(64),
          participant_values_digest: 'f'.repeat(64),
          certificate: {
            der_base64: body.certificate_base64,
            fingerprint: 'a'.repeat(64),
            summary: {
              subject: 'Test',
              issuer: 'CA interna',
              serial_hex: '01',
              not_before_unix: 1,
              not_after_unix: 2000000000,
            },
          },
          deployment_id: caseId,
          trust_revision: 1,
          root_fingerprint: 'b'.repeat(64),
          policy: 'internal_demo_v1',
        };
      }
      return route.fulfill({
        json: {
          case_id: caseId,
          ...body,
          declaration,
          submission_revision: body.proposal.expected_participant_revision + 1,
          submission_digest: declaration ? null : 'd'.repeat(64),
        },
      });
    }
    const proposal = body.prepared.proposal,
      identity = state.proposedSubject;
    const record = {
      ...typed,
      ...proposal.values.role,
      id: proposal.participant_id,
      revision: proposal.expected_participant_revision + 1,
      display_name: identity.values.name.value || identity.values.name,
      procedural_role: proposal.values.role.profile.kind,
      subject: identity,
      credential_origin: body.signature_base64
        ? {
            participant_id: proposal.participant_id,
            participant_revision: proposal.expected_participant_revision + 1,
            statement_digest: 'e'.repeat(64),
          }
        : null,
    };
    state.records.set(record.id, [...(state.records.get(record.id) || []), record]);
    return route.fulfill({
      status: proposal.expected_participant_revision ? 200 : 201,
      json: record,
    });
  });
  return state;
}
export async function pickSupport(page, label) {
  const field = page.getByRole('group', { name: label, exact: true });
  await field.getByRole('button', { name: 'Seleccionar documento', exact: true }).click();
  const picker = field.getByRole('region', { name: 'Seleccionar soporte exacto' });
  await picker.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1 \/ contrato.pdf/ }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await field.getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }).fill('Pagina 1');
}
export async function fillTyped(page, kind = 'defendant') {
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Agregar participante tipificado' });
  await dialog.getByLabel('Nombre de la persona', { exact: true }).fill('Persona tipificada');
  await dialog.getByLabel('Estado de CURP', { exact: true }).selectOption('unknown');
  await dialog.getByLabel('Motivo de CURP', { exact: true }).fill('No consta en el soporte');
  await pickSupport(page, 'Soporte de identidad');
  await dialog.getByLabel('Tipo de participante', { exact: true }).selectOption(kind);
  if (kind === 'defendant') {
    await dialog
      .getByLabel('Estado de Situaci\u00f3n de libertad declarada')
      .selectOption('unknown');
    await dialog
      .getByLabel('Motivo de Situaci\u00f3n de libertad declarada')
      .fill('Sin dato comprobado');
  } else if (kind === 'control_judge')
    await dialog
      .getByLabel('\u00d3rgano jurisdiccional', { exact: true })
      .fill('Juzgado de control');
  await pickSupport(page, 'Soporte del rol');
  return dialog;
}
export async function prepareTyped(dialog) {
  await dialog
    .getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true })
    .click();
  await expect(
    dialog.getByRole('heading', { name: 'Revisi\u00f3n de identidad', exact: true }),
  ).toBeVisible();
  await dialog
    .getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true })
    .fill('Identidad revisada con el documento');
  await dialog.getByRole('button', { name: 'Preparar registro', exact: true }).click();
}
