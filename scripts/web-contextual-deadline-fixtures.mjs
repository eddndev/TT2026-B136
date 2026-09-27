// Reuse existing actors and calendar while isolating contextual creation data.
import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { persistFollowRecord as persist } from './web-deadline-reevaluation.mjs';
import { profileDefinition } from './web-deadline-definitions.mjs';

const unknown = () => ({ kind: 'unknown', reason: 'Dato no declarado en esta prueba' });
export async function provisionContextualDeadline(call, accounts, calendar, future) {
  const reference = 'RESOURCE-CONTEXTUAL-DEADLINE';
  const created = await call('POST', '/penal-cases', {
    title: 'Creacion contextual de plazo', reference,
    profile: { nuc: reference + '-NUC', nuc_authority: 'Autoridad sintetica',
      judicial_case_number: reference + '-CJ', judicial_authority: 'Organo sintetico',
      offenses: ['Descripcion sintetica'], general_information: null,
      complementary_identifiers: null },
  }, 201);
  const scenario = { case: { id: created.id, title: created.administration.title, reference } };
  const route = `/cases/${created.id}`;
  await call('PUT', `${route}/members/${accounts.owner.id}`, undefined, 204);
  const pdf = await readFile(new URL('../crates/infrastructure/tests/fixtures/stage-support.pdf', import.meta.url));
  const support = await call('POST', route + '/documents', pdf, 201,
    { 'X-Document-Name': 'contextual-deadline-source.pdf' });
  const evidence = { document_id: support.id, version: support.version,
    digest: support.digest, locator: 'Pagina 1 declarada' };
  const at = { precision: 'date', year: future.getUTCFullYear(),
    month: future.getUTCMonth() + 1, day: future.getUTCDate(), offset_seconds: null };
  scenario.source = await persist(call, route + '/resolutions', {
    family: 'resolution', id: randomUUID(), operation_id: randomUUID(),
    change: { action: 'record', expected_revision: 0, values: {
      class: { kind: 'known', value: { kind: 'order' } }, subtype: null,
      issuer: { kind: 'known', value: 'Autoridad sintetica' }, issued_at: at,
      summary: 'Resolucion activa para el plazo contextual',
      provenance: { kind: 'external_reference', reference: 'Constancia sintetica', support: evidence },
    } },
  });
  scenario.resourceInitial = await persist(call, route + '/procedural-resources', {
    operation_id: randomUUID(), resource_id: randomUUID(), change: {
      action: 'register', expected_revision: 0, values: {
        kind: 'revocation', mode: { kind: 'known', value: 'written' },
        title: 'Recurso con creacion contextual de plazo',
        resolution: { id: scenario.source.id, revision: 1 }, resolution_evidence: evidence,
        resolution_reference: unknown(), issuing_authority: unknown(), receiving_authority: null,
        resolution_at: at, notification_at: null, challenged_part: 'Apartado declarado',
        grounds: 'Motivos sinteticos sin calificacion juridica',
        appellants: [{ name: 'Persona recurrente contextual', role: unknown(), participant: null }],
      },
    },
  });
  scenario.resourceAct = await persist(call, route + '/procedural-resources', {
    operation_id: randomUUID(), resource_id: scenario.resourceInitial.id, change: {
      action: 'record_act', expected_revision: 1, act_id: randomUUID(), values: {
        kind: 'interposition', mode: { kind: 'known', value: 'written' }, occurred_at: at,
        authority: unknown(), statement: 'Acto declarado para una actividad contextual',
        evidence: [evidence],
      },
    },
  }, 'POST', `/${scenario.resourceInitial.id}/acts`);
  scenario.resource = scenario.resourceAct;
  const definition = profileDefinition(created.id, 'daily', calendar.values);
  definition.title = 'Dias contextuales de prueba';
  definition.completion.from = calendar.values.coverage.from;
  definition.completion.through = calendar.values.coverage.through;
  scenario.profile = await persist(call, route + '/deadline-profiles', {
    operation_id: randomUUID(), profile_id: randomUUID(),
    change: { action: 'publish', expected_revision: 0, definition },
  });
  scenario.calendar = calendar;
  scenario.date = future.toISOString().slice(0, 10);
  scenario.dueAt = { unix_seconds: Math.floor(Date.UTC(at.year, at.month - 1, at.day, 23, 30) / 1000),
    nanosecond: 0, offset_seconds: 0 };
  return scenario;
}
