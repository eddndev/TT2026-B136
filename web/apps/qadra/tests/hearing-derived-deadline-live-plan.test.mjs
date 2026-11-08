import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planLiveSuite } from '../../../../scripts/web-live-plan.mjs';

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
