import { readFile } from 'node:fs/promises';
import { test, expect } from '@playwright/test';
import { observeFrontendEntry } from '../frontend-entry-diagnostics.mjs';

const privateMarkers = {
  query: 'PRIVATE_QUERY_SENTINEL',
  fragment: 'PRIVATE_FRAGMENT_SENTINEL',
  input: 'PRIVATE_INPUT_SENTINEL@example.test',
  path: 'PRIVATE_PATH_SENTINEL',
  assetQuery: 'PRIVATE_ASSET_QUERY_SENTINEL',
  assetFragment: 'PRIVATE_ASSET_FRAGMENT_SENTINEL',
  message: 'PRIVATE_HYDRATION_MESSAGE_SENTINEL',
  stack: 'PRIVATE_HYDRATION_STACK_SENTINEL',
};

async function enter(page) {
  const response = await page.goto(
    `/?diagnostic=${privateMarkers.query}#${privateMarkers.fragment}`,
  );
  expect(response.status()).toBe(200);
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await expect(page.locator('astro-island')).not.toHaveAttribute('ssr', '');
  await page.getByLabel('Correo electr\u00f3nico').fill(privateMarkers.input);
  await expect(page.getByLabel('Correo electr\u00f3nico')).toHaveValue(privateMarkers.input);
}

async function hydrationFailure(page, count = 1) {
  await page.evaluate(
    ({ markers, count }) => {
      const island = document.querySelector('astro-island');
      const component = new URL(island.getAttribute('component-url'), location.href);
      component.search = `token=${markers.query}`;
      component.hash = markers.fragment;
      for (let index = 0; index < count; index++) {
        const error = new TypeError(markers.message);
        error.stack = markers.stack;
        island.dispatchEvent(
          new CustomEvent('astro:hydration-error', {
            bubbles: true,
            detail: { componentUrl: component.href, error },
          }),
        );
      }
    },
    { markers: privateMarkers, count },
  );
}

async function missingScripts(page, count = 1) {
  await page.evaluate(
    async ({ markers, count }) => {
      await Promise.all(
        Array.from(
          { length: count },
          (_, index) =>
            new Promise((resolve) => {
              const script = document.createElement('script');
              script.type = 'module';
              script.src =
                `/__entry_diagnostic_missing__/${markers.path}-${index}.mjs` +
                `?token=${markers.assetQuery}#${markers.assetFragment}`;
              script.onload = script.onerror = () => {
                script.remove();
                resolve();
              };
              document.head.append(script);
            }),
        ),
      );
    },
    { markers: privateMarkers, count },
  );
}

function exactKeys(value, names) {
  expect(Object.keys(value).sort()).toEqual([...names].sort());
}

function publicSnapshot(value) {
  exactKeys(value, ['version', 'document', 'islands', 'failures', 'hydrationErrors', 'dropped']);
  expect(value.version).toBe(1);
  exactKeys(value.document, ['available', 'readyState']);
  expect(value.document.available).toBe(true);
  expect(['interactive', 'complete']).toContain(value.document.readyState);
  expect(value.islands.length).toBeLessThanOrEqual(4);
  for (const island of value.islands) {
    exactKeys(island, ['component', 'client', 'ssr', 'children']);
    expect(typeof island.component).toBe('string');
    expect(typeof island.ssr).toBe('boolean');
    expect(Number.isInteger(island.children)).toBe(true);
    expect(island.children).toBeGreaterThanOrEqual(0);
    expect(island.children).toBeLessThanOrEqual(1000);
  }
  expect(value.failures.length).toBeLessThanOrEqual(12);
  for (const failure of value.failures) {
    exactKeys(failure, ['resource', 'type', 'status', 'failure']);
    expect(['document', 'script', 'stylesheet']).toContain(failure.type);
    expect([null, 'aborted', 'connection', 'timeout', 'other']).toContain(failure.failure);
    if (failure.status !== null) {
      expect(Number.isInteger(failure.status)).toBe(true);
      expect(failure.status).toBeGreaterThanOrEqual(400);
      expect(failure.status).toBeLessThanOrEqual(599);
    }
  }
  expect(value.hydrationErrors.length).toBeLessThanOrEqual(4);
  for (const failure of value.hydrationErrors) {
    exactKeys(failure, ['component', 'error']);
    expect(['Error', 'TypeError', 'SyntaxError', 'ReferenceError', 'other']).toContain(
      failure.error,
    );
  }
  exactKeys(value.dropped, ['failures', 'hydrationErrors', 'islands']);
  for (const count of Object.values(value.dropped)) {
    expect(Number.isInteger(count)).toBe(true);
    expect(count).toBeGreaterThanOrEqual(0);
  }
  const encoded = JSON.stringify(value);
  expect(Buffer.byteLength(encoded)).toBeLessThanOrEqual(8192);
  for (const marker of Object.values(privateMarkers)) expect(encoded).not.toContain(marker);
  expect(encoded).not.toContain('http://');
  expect(encoded).not.toContain('https://');
  return encoded;
}

test('entry diagnostics observe real Astro startup and failures without copying private inputs', async ({
  page,
}, testInfo) => {
  const observer = await observeFrontendEntry(page);
  try {
    await enter(page);
    const initial = await observer.snapshot();
    publicSnapshot(initial);
    expect(initial.islands).toContainEqual({
      component: '/src/components/App.svelte',
      client: 'only',
      ssr: false,
      children: expect.any(Number),
    });
    expect(initial.failures).toEqual([]);
    expect(initial.hydrationErrors).toEqual([]);

    const response = page.waitForResponse((value) =>
      new URL(value.url()).pathname.startsWith('/__entry_diagnostic_missing__/'),
    );
    await missingScripts(page);
    expect((await response).status()).toBe(404);
    await hydrationFailure(page);
    await expect.poll(async () => (await observer.snapshot()).failures.length).toBeGreaterThan(0);
    const snapshot = await observer.snapshot();
    publicSnapshot(snapshot);
    expect(snapshot.failures).toContainEqual({
      resource: 'other',
      type: 'script',
      status: 404,
      failure: null,
    });
    expect(snapshot.hydrationErrors).toEqual([
      { component: '/src/components/App.svelte', error: 'TypeError' },
    ]);
    await observer.attach(testInfo);
    const artifacts = testInfo.attachments.filter((value) => value.name === 'frontend-entry');
    expect(artifacts).toHaveLength(1);
    expect(artifacts[0].contentType).toBe('application/json');
    const bytes = artifacts[0].body ?? (await readFile(artifacts[0].path));
    expect(bytes.length).toBeLessThanOrEqual(8192);
    const encoded = bytes.toString('utf8');
    for (const marker of Object.values(privateMarkers)) expect(encoded).not.toContain(marker);
    expect(JSON.parse(encoded)).toEqual(snapshot);
  } finally {
    await observer.dispose();
  }
});

test('entry diagnostics bound repeated failures and stop observing when disposed', async ({
  page,
}) => {
  const observer = await observeFrontendEntry(page);
  try {
    await enter(page);
    await missingScripts(page, 14);
    await hydrationFailure(page, 20);
    await expect.poll(async () => (await observer.snapshot()).failures.length).toBe(12);
    const before = await observer.snapshot();
    publicSnapshot(before);
    expect(before.dropped.failures).toBeGreaterThanOrEqual(2);
    expect(before.hydrationErrors).toHaveLength(4);
    expect(before.dropped.hydrationErrors).toBe(16);
    await observer.dispose();
    await hydrationFailure(page);
    const after = await observer.snapshot();
    expect(after.failures).toEqual(before.failures);
    expect(after.hydrationErrors).toEqual(before.hydrationErrors);
    expect(after.dropped).toEqual(before.dropped);
    publicSnapshot(after);
  } finally {
    await observer.dispose();
  }
});
