import test from 'node:test';
import assert from 'node:assert/strict';
import {
  subjectDraft,
  subjectValues,
  roleDraft,
  roleValues,
  reviewValues,
  validateSupportSet,
  profileKinds,
} from '../src/lib/typed-participant-values.mjs';
const support = { document_id: 'doc', version: 1, digest: 'a'.repeat(64), locator: 'Pagina 1' };
const unknown = { state: 'unknown', reason: 'No disponible en el soporte' };
const license = { number: '000123', issuer: 'SEP' };

test('represented identity keeps explicit unknown/unidentified values and exact document section', () => {
  const draft = subjectDraft();
  draft.name = {
    state: 'unidentified',
    label: 'Persona por identificar',
    reason: 'Sin nombre en acta',
  };
  draft.curp = unknown;
  draft.identity_support = support;
  assert.deepEqual(subjectValues(draft), draft);
  draft.name = { state: 'known', value: ' Ana \u00c1vila ' };
  draft.curp = { state: 'known', value: 'abcd001122hmnrrr01' };
  assert.equal(subjectValues(draft).name.value, 'Ana \u00c1vila');
  assert.equal(subjectValues(draft).curp.value, 'ABCD001122HMNRRR01');
  draft.curp = { state: 'unknown', reason: '' };
  assert.throws(() => subjectValues(draft), /motivo/i);
});

test('institutional subjects omit personal data and never invent a CURP', () => {
  const value = subjectValues({
    kind: 'institutional_body',
    name: 'Tribunal',
    institutional_identifier: unknown,
    identity_support: support,
    curp: { state: 'known', value: 'irrelevant' },
  });
  assert.equal(value.kind, 'institutional_body');
  assert.equal('curp' in value, false);
  assert.equal(typeof value.name, 'string');
});

test('all eleven profiles serialize only fields of their variant and preserve license zeros', () => {
  const samples = {
    defendant: { custody: { state: 'known', value: 'detained' } },
    victim: {
      contact: { state: 'not_recorded', reason: 'Sin contacto registrado' },
      protection: { state: 'documented', support },
    },
    defense_counsel: { license, mode: 'private' },
    prosecutor: { office_identifier: unknown, unit: unknown, license: unknown },
    victim_counsel: { institution: 'Despacho', license: { state: 'known', value: license } },
    control_judge: { court: 'Juzgado' },
    trial_court: { judicial_district: 'Distrito', composition: 'collegiate' },
    expert: { specialty: unknown, license: unknown },
    police: { agency: unknown, unit: unknown },
    precautionary_supervisor: { authority: unknown, unit: unknown },
    other: { label: 'Testigo', description: unknown },
  };
  assert.deepEqual(
    profileKinds.map((item) => item.key),
    Object.keys(samples),
  );
  for (const [kind, fields] of Object.entries(samples)) {
    const draft = roleDraft(kind);
    draft.role_support = support;
    draft.profile = { kind, ...fields, stale_field: 'Must not be sent' };
    assert.deepEqual(roleValues(draft).profile, { kind, ...fields });
  }
});

test('typed fields enforce scalar bounds, reject controls before trim and require support', () => {
  const draft = roleDraft('control_judge');
  draft.profile.court = 'A'.repeat(200);
  draft.role_support = support;
  assert.equal(roleValues(draft).profile.court.length, 200);
  for (const value of ['\tJuzgado', 'A'.repeat(201), '']) {
    draft.profile.court = value;
    assert.throws(() => roleValues(draft));
  }
  draft.profile.court = 'Juzgado';
  draft.role_support = null;
  assert.throws(() => roleValues(draft), /soporte/i);
  const victim = roleDraft('victim');
  victim.role_support = support;
  victim.profile.contact = { state: 'unknown', reason: 'Desconocido' };
  victim.profile.protection = { state: 'none_declared', reason: '' };
  assert.throws(() => roleValues(victim), /motivo/i);
});

test('candidate review requires an explicit decision for every current candidate', () => {
  const reference = { kind: 'subject', id: 'candidate', revision: 2 };
  const review = {
    directory_stamp: 'b'.repeat(64),
    candidates: [{ reference, display_name: 'Ana' }],
  };
  assert.throws(() => reviewValues(review, 'Seleccion elegida', {}), /candidato/i);
  const decisions = {
    'subject:candidate:2': { reason: 'Persona distinta segun soporte', support },
  };
  assert.deepEqual(reviewValues(review, ' Seleccion elegida ', decisions), {
    directory_stamp: review.directory_stamp,
    selection_reason: 'Seleccion elegida',
    different: [{ candidate: reference, reason: decisions['subject:candidate:2'].reason, support }],
  });
  review.candidates[0].reference.revision = 3;
  assert.throws(() => reviewValues(review, 'Seleccion elegida', decisions), /candidato/i);
});

test('support capacity counts exact unique versions and rejects inconsistent repeated digests', () => {
  assert.doesNotThrow(() =>
    validateSupportSet(
      support,
      { ...support, locator: 'Otra seccion' },
      { ...support, document_id: 'second' },
    ),
  );
  assert.throws(
    () =>
      validateSupportSet(
        support,
        { ...support, document_id: 'second' },
        { ...support, version: 2 },
      ),
    /dos/i,
  );
  assert.throws(
    () => validateSupportSet(support, { ...support, digest: 'b'.repeat(64) }),
    /digest/i,
  );
});

test('does not turn non-ASCII CURP characters into a different ASCII identifier', () => {
  const draft = subjectDraft();
  draft.name = { state: 'known', value: 'Nombre' };
  draft.curp = { state: 'known', value: '\u00df' + 'A'.repeat(16) };
  draft.identity_support = {
    document_id: 'document',
    version: 1,
    digest: 'a'.repeat(64),
    locator: 'Pagina 1',
  };
  assert.throws(() => subjectValues(draft), /CURP/);
});
