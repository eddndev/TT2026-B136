// Independent resource captures through the authenticated disposable HTTP service.
import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const envelope = (draft) => ({ command: draft.command, expected_submission_digest: draft.submission_digest });
const unknown = () => ({ kind: 'unknown', reason: 'No consta en la captura sintetica' });
export async function provisionResources(call) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL) throw new Error('disposable browser services are required');
  const principal = await call('GET', '/auth/me');
  if (principal.role !== 'owner' || !principal.id || !principal.email)
    throw new Error('resource fixture requires the authenticated owner principal');
  const fixture = {};
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    const password = `procedural resources browser ${role} password`;
    const enrollment = await call('POST', '/users', {
      email: `procedural-resources.${role}@example.com`, password, role,
    }, 201);
    fixture[role] = { id: enrollment.user.id, email: enrollment.user.email,
      password, recoveryCodes: enrollment.recovery_codes };
  }
  const pdf = await readFile(new URL('../crates/infrastructure/tests/fixtures/stage-support.pdf', import.meta.url));
  for (const key of ['desktop', 'mobile', 'policy']) {
    const reference = `RESOURCE-${key.toUpperCase()}`;
    const row = await call('POST', '/penal-cases', {
      title: `Recursos declarados ${key}`, reference,
      profile: {
        nuc: `${reference}-NUC`, nuc_authority: 'Autoridad declarada',
        judicial_case_number: `${reference}-CJ`, judicial_authority: 'Organo declarado',
        offenses: ['Descripcion sintetica'], general_information: null,
        complementary_identifiers: null,
      },
    }, 201);
    const { title, reference: capturedReference } = row.administration;
    if (!title || capturedReference !== reference)
      throw new Error('resource case response lacks its actual administrative metadata');
    const scenario = { case: { id: row.id, title, reference: capturedReference } };
    const path = `/cases/${row.id}`;
    for (const role of ['litigator', 'paralegal', 'client'])
      await call('PUT', `${path}/members/${fixture[role].id}`, undefined, 204);
    scenario.support = await call('POST', `${path}/documents`, pdf, 201,
      { 'X-Document-Name': `resource-${key}-source.pdf` });
    const evidence = { document_id: scenario.support.id, version: scenario.support.version,
      digest: scenario.support.digest, locator: 'Pagina 1 historica' };
    const factCommand = {
      family: 'resolution', operation_id: randomUUID(), id: randomUUID(),
      change: { action: 'record', expected_revision: 0, values: {
        class: { kind: 'known', value: { kind: 'order' } }, subtype: null, issuer: unknown(),
        issued_at: { precision: 'date', year: 2026, month: 9, day: 18, offset_seconds: null },
        summary: `Resolucion historica ${key}`,
        provenance: { kind: 'external_reference', reference: 'Constancia sintetica', support: evidence },
      } },
    };
    const factDraft = await call('POST', `${path}/resolutions/prepare`, factCommand);
    scenario.resolution = await call('POST', `${path}/resolutions`, envelope(factDraft), 201);
    const withdrawal = await call('POST', `${path}/resolutions/prepare`, {
      family: 'resolution', operation_id: randomUUID(), id: scenario.resolution.id,
      change: { action: 'withdraw', expected_revision: 1,
        reason: 'Retiro organizativo conservando la fuente historica' },
    });
    scenario.resolutionHead = await call('POST', `${path}/resolutions/${scenario.resolution.id}/withdrawal`, envelope(withdrawal), 201);
    scenario.currentSupport = await call('POST', `${path}/documents/${scenario.support.id}/versions?expected_version=1`, pdf, 201,
      { 'X-Document-Name': `resource-${key}-current.pdf` });
    const resourceCommand = {
      operation_id: randomUUID(), resource_id: randomUUID(),
      change: { action: 'register', expected_revision: 0, values: {
        kind: key === 'mobile' ? 'appeal' : 'revocation', mode: { kind: 'known', value: 'written' },
        title: `Recurso con fuente historica ${key}`,
        resolution: { id: scenario.resolution.id, revision: 1 }, resolution_evidence: evidence,
        resolution_reference: unknown(), issuing_authority: unknown(), receiving_authority: null,
        resolution_at: structuredClone(scenario.resolution.values.issued_at), notification_at: null,
        challenged_part: 'Apartado declarado como impugnado', grounds: 'Motivos declarados por el operador',
        appellants: [{ name: `Persona recurrente ${key}`, role: unknown(), participant: null }],
      } },
    };
    const draft = await call('POST', `${path}/procedural-resources/prepare`, resourceCommand);
    if (draft.recorded_by.id !== principal.id || draft.recorded_by.email !== principal.email)
      throw new Error('resource preparation does not capture its authenticated author');
    scenario.resource = await call('POST', `${path}/procedural-resources`, envelope(draft), 201);
    if (scenario.resource.revision !== 1 || scenario.resource.sources.resolution.revision !== 1
      || scenario.resource.sources.supports[0].version !== 1)
      throw new Error('resource fixture did not retain the selected historical sources');
    fixture[key] = scenario;
  }
  return fixture;
}
