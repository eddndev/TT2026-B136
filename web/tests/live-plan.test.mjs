import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import * as planning from '../../scripts/web-live-plan.mjs';
import { planLiveSuite, fixtureNames } from '../../scripts/web-live-plan.mjs';

const files = readdirSync(new URL('./live/', import.meta.url)).filter((f) =>
  f.endsWith('.spec.mjs'),
);

test('live partitions cover every current spec exactly once without a fixed test count', () => {
  const plans = [1, 2, 3].map((n) => planLiveSuite(files, `${n}/3`));
  const selected = plans.flatMap((p) => p.files);
  assert.deepEqual([...selected].sort(), [...files].sort());
  assert.equal(new Set(selected).size, files.length);
  assert.ok(plans.every((p) => p.files.length > 0));
});

test('related scenarios share expensive fixtures and preserve dependent setup', () => {
  const related = [
    'case-stages.spec.mjs',
    'stage-adoption.spec.mjs',
    'combined-agenda.spec.mjs',
    'alerts.spec.mjs',
    'case-participants.spec.mjs',
    'typed-participants.spec.mjs',
  ];
  const plans = [1, 2, 3].map((n) => planLiveSuite(related, `${n}/3`));
  assert.ok(plans[0].fixtures.includes('caseStages'));
  assert.ok(plans[0].fixtures.includes('deadlineReevaluation'));
  assert.ok(plans[0].fixtures.includes('combinedAgenda'));
  assert.ok(!plans[0].fixtures.includes('participants'));
  assert.ok(plans[1].fixtures.includes('alerts'));
  assert.ok(plans[1].fixtures.includes('hearings'));
  assert.ok(!plans[1].fixtures.includes('caseStages'));
  assert.ok(plans[2].files.includes('case-participants.spec.mjs'));
  assert.ok(plans[2].files.includes('typed-participants.spec.mjs'));
  assert.ok(plans[2].fixtures.includes('participants'));
});

test('administration runs with participants on the primary without unrelated stage setup', () => {
  const plans = [1, 2, 3].map((n) => planLiveSuite(files, `${n}/3`));
  const owner = plans.find((plan) => plan.files.includes('case-participants.spec.mjs'));
  assert.ok(owner.files.includes('case-administration.spec.mjs'));
  assert.ok(owner.fixtures.includes('caseAdministration'));
  assert.ok(!owner.fixtures.includes('caseStages'));
  assert.equal(plans.filter((plan) => plan.fixtures.includes('caseAdministration')).length, 1);
});

test('future unclassified specs run once with conservative complete fixtures', () => {
  const future = 'new-feature.spec.mjs';
  const plans = [1, 2, 3].map((n) => planLiveSuite([...files, future], `${n}/3`));
  const owners = plans.filter((p) => p.files.includes(future));
  assert.equal(owners.length, 1);
  assert.deepEqual(owners[0].fixtures, fixtureNames);
});

test('unpartitioned execution retains the complete fixture setup', () => {
  const plan = planLiveSuite(files);
  assert.deepEqual(plan.files, [...files].sort());
  assert.deepEqual(plan.fixtures, fixtureNames);
});

test('bad partition inputs fail instead of silently skipping specs', () => {
  for (const value of ['0/3', '4/3', '1/2', '1', 'all', '../3']) {
    assert.throws(() => planLiveSuite(files, value), /partition/);
  }
});

test('discovery includes nested specs and future supported test extensions', () => {
  const directory = mkdtempSync(join(tmpdir(), 'tt-live-discovery-'));
  try {
    mkdirSync(join(directory, 'nested'));
    for (const file of ['existing.spec.mjs', 'nested/new.test.ts', 'nested/helpers.mjs'])
      writeFileSync(join(directory, file), '');
    assert.deepEqual(planning.discoverLiveSpecs(directory), [
      'existing.spec.mjs',
      'nested/new.test.ts',
    ]);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('real document admission shares only the exact content fixture family', () => {
  const file = 'document-admission-real.spec.mjs';
  const plans = [1, 2, 3].map((n) => planLiveSuite([file], `${n}/3`));
  assert.deepEqual(plans[1], { files: [file], fixtures: ['documentContent'] });
  assert.deepEqual(plans[0].files, []);
  assert.deepEqual(plans[2].files, []);
});
