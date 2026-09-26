import test from 'node:test';
import assert from 'node:assert/strict';
import { provisionFixtures } from '../../scripts/web-fixture-provisioning.mjs';

test('fixture provisioning bounds concurrency and preserves named results', async () => {
  let active = 0;
  let peak = 0;
  const tasks = Object.fromEntries(
    ['first', 'second', 'third', 'fourth'].map((key) => [
      key,
      async () => {
        active += 1;
        peak = Math.max(peak, active);
        await new Promise((resolve) => setTimeout(resolve, 5));
        active -= 1;
        return key;
      },
    ]),
  );
  assert.deepEqual(await provisionFixtures(tasks, 2), {
    first: 'first',
    second: 'second',
    third: 'third',
    fourth: 'fourth',
  });
  assert.equal(peak, 2);
  assert.equal(active, 0);
});

test('local fixture provisioning defaults to one active task', async () => {
  const calls = [];
  const task = (name) => async () => {
    calls.push(`start ${name}`);
    await Promise.resolve();
    calls.push(`end ${name}`);
  };
  await provisionFixtures({ first: task('first'), second: task('second') });
  assert.deepEqual(calls, ['start first', 'end first', 'start second', 'end second']);
});

test('a failed fixture drains the active batch and never starts later batches', async () => {
  let completed = false;
  let later = false;
  await assert.rejects(
    provisionFixtures(
      {
        bad: async () => {
          throw new Error('fixture failed');
        },
        active: async () => {
          await new Promise((resolve) => setTimeout(resolve, 10));
          completed = true;
        },
        later: async () => {
          later = true;
        },
      },
      2,
    ),
    /fixture failed/,
  );
  assert.equal(completed, true);
  assert.equal(later, false);
});

test('invalid fixture concurrency is rejected before any work starts', async () => {
  for (const workers of [0, -1, 3, 1.5, NaN]) {
    await assert.rejects(provisionFixtures({}, workers), /one or two/);
  }
});
