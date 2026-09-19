import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  resourcePreparedValue,
  resourceRecordValue,
} from '../src/lib/procedural-resource-validation.mjs';
import {
  resourceCaseId,
  resourcePrepared,
  resourceRecord,
} from './fixtures/procedural-resource-unit.mjs';

const actor = {
  id: '00000000-0000-0000-0000-000000000003',
  email: 'owner@example.test',
};
const at = '2023-11-14T22:13:20Z';
const administrationDigest = '11'.repeat(32);
const support = {
  document_id: '00000000-0000-0000-0000-000000000002',
  version: 1,
  digest: 'ab'.repeat(32),
};

// These capture shapes follow crates/web/tests/case_stage_support/mod.rs.
function initial() {
  return {
    kind: 'initial',
    case_id: resourceCaseId,
    stage_revision: 1,
    stage: 'investigation',
    administration_revision: 1,
    administration_digest: administrationDigest,
    recorded_at: at,
    recorded_by: { ...actor },
  };
}
function changed() {
  return {
    ...initial(),
    kind: 'change',
    stage_revision: 2,
    stage: 'intermediate',
    from_stage: 'investigation',
    values_digest: '22'.repeat(32),
    values: {
      kind: 'to_intermediate',
      accusation_declared_at: {
        precision: 'instant',
        at: '2023-02-01T10:11:12.125-06:00',
      },
      accusation: { ...support },
      note: 'Nota',
    },
    supports: [{ ...support, name: 'acta.pdf', format: 'pdf', policy: 'pdf_docx_v1' }],
  };
}
function prepared(current = initial()) {
  const value = resourcePrepared();
  value.observed_administration = {
    kind: 'recorded',
    case_id: resourceCaseId,
    revision: 1,
    title: 'Expediente declarado',
    reference: 'REF-1',
    status: 'active',
    values_digest: administrationDigest,
    changed_at: at,
    changed_by: { ...actor },
  };
  value.observed_stage = { case_id: resourceCaseId, current };
  return value;
}
function accepts(value) {
  assert.equal(resourcePreparedValue(value), value);
  const row = resourceRecord(value);
  assert.equal(resourceRecordValue(row, resourceCaseId, row.id, row.revision), row);
}
function rejects(value) {
  const row = resourceRecord(value);
  assert.throws(() => resourcePreparedValue(value));
  assert.throws(() => resourceRecordValue(row, resourceCaseId, row.id, row.revision));
}

test('resource context preserves an initial stage with its exact administrative capture', () => {
  accepts(prepared());
});
test('resource context preserves a complete changed stage and its documentary capture', () => {
  accepts(prepared(changed()));
});
test('a later administration can preserve an earlier stage with a different historical digest', () => {
  const value = prepared(changed());
  value.observed_administration.revision = 2;
  value.observed_administration.values_digest = '33'.repeat(32);
  accepts(value);
});
test('resource context rejects a registered stage with an unrevised administration', () => {
  const value = prepared();
  value.observed_administration = resourcePrepared().observed_administration;
  rejects(value);
});
test('resource context rejects a stage from a future administrative revision', () => {
  const value = prepared(changed());
  value.observed_stage.current.administration_revision = 2;
  rejects(value);
});
test('resource context rejects different digests for the same administrative revision', () => {
  const value = prepared();
  value.observed_stage.current.administration_digest = '44'.repeat(32);
  rejects(value);
});
for (const field of ['from_stage', 'values', 'supports']) {
  test(`a changed stage requires its ${field} capture`, () => {
    const value = prepared(changed());
    delete value.observed_stage.current[field];
    rejects(value);
  });
}
