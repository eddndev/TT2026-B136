import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planLiveSuite } from '../../scripts/web-live-plan.mjs';

const ownerFile = 'owner-certificates.spec.mjs';
const partitions = ['1/3', '2/3', '3/3'];

test('real Owner certificates use only their own fixture family on the primary runner', () => {
  const plans = partitions.map((partition) => planLiveSuite([ownerFile], partition));
  assert.deepEqual(plans[0], { files: [], fixtures: [] });
  assert.deepEqual(plans[1], { files: [], fixtures: [] });
  assert.deepEqual(plans[2], { files: [ownerFile], fixtures: ['ownerCertificates'] });
});

test('adding the Owner journey preserves existing assignments and fixture ownership', () => {
  const existing = [
    'document-workflow.spec.mjs',
    'members.spec.mjs',
    'judicial-calendars.spec.mjs',
    'case-stages.spec.mjs',
    'typed-participants.spec.mjs',
  ];
  const expectedFiles = [
    ['document-workflow.spec.mjs'],
    ['judicial-calendars.spec.mjs', 'members.spec.mjs'],
    ['case-stages.spec.mjs', 'typed-participants.spec.mjs'],
  ];
  for (const [index, partition] of partitions.entries()) {
    const before = planLiveSuite(existing, partition);
    const after = planLiveSuite([...existing, ownerFile], partition);
    assert.deepEqual(before.files, expectedFiles[index]);
    for (const file of before.files) assert.ok(after.files.includes(file));
    assert.deepEqual(
      after.files.filter((file) => file !== ownerFile),
      before.files,
    );
    assert.deepEqual(
      after.fixtures.filter((family) => family !== 'ownerCertificates'),
      before.fixtures,
    );
    assert.equal(after.fixtures.includes('ownerCertificates'), index === 2);
  }
});
