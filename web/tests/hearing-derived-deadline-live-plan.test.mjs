import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fixtureNames, planLiveSuite } from '../../scripts/web-live-plan.mjs';

const file = 'hearing-derived-deadline-real.spec.mjs';
const partitions = ['1/3', '2/3', '3/3'];

test('derived hearing deadlines select their own fixture only in partition three', () => {
  for (const name of [file, `nested/${file}`]) {
    const plans = partitions.map((partition) => planLiveSuite([name], partition));
    assert.deepEqual(plans[0], { files: [], fixtures: [] });
    assert.deepEqual(plans[1], { files: [], fixtures: [] });
    assert.deepEqual(plans[2], {
      files: [name],
      fixtures: ['hearingDerivedDeadlines'],
    });
  }
});

test('the compound journey preserves existing hearing and deadline ownership', () => {
  const existing = [
    'deadline-workflow.spec.mjs',
    'hearing-result-workflow.spec.mjs',
    'hearings.spec.mjs',
    'resource-hearings-real.spec.mjs',
  ];
  const expectedFiles = [[existing[0]], [existing[1], existing[2]], [existing[3]]];
  for (const [index, partition] of partitions.entries()) {
    const before = planLiveSuite(existing, partition);
    const after = planLiveSuite([...existing, file], partition);
    assert.deepEqual(before.files, expectedFiles[index]);
    assert.deepEqual(
      after.files.filter((name) => name !== file),
      before.files,
    );
    assert.deepEqual(
      after.fixtures.filter((name) => name !== 'hearingDerivedDeadlines'),
      before.fixtures,
    );
    assert.equal(after.fixtures.includes('hearingDerivedDeadlines'), index === 2);
  }
});

test('unpartitioned execution keeps the complete fixture inventory including the compound journey', () => {
  const plan = planLiveSuite([file]);
  assert.deepEqual(plan.files, [file]);
  assert.deepEqual(plan.fixtures, fixtureNames);
  assert.equal(new Set(plan.fixtures).size, plan.fixtures.length);
  for (const name of [
    'participants',
    'caseAdministration',
    'caseStages',
    'hearings',
    'alerts',
    'hearingResults',
    'proceduralFacts',
    'proceduralResources',
    'resourceActivities',
    'documentContent',
    'members',
    'dashboard',
    'caseReports',
    'auditEvents',
    'judicialCalendars',
    'deadlines',
    'deadlineReevaluation',
    'combinedAgenda',
    'ownerCertificates',
    'hearingDerivedDeadlines',
  ]) {
    assert.ok(plan.fixtures.includes(name), `missing fixture family: ${name}`);
  }
});
