import { test, expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import {
  setupResourceDeadline,
  fillDeadline,
  editor,
  prepareResourceDeadline,
} from './resource-deadline-helpers.mjs';

test('a slow valid contextual preparation reaches review after one exact request', async ({
  page,
}) => {
  const state = await setupResourceDeadline(page);
  const path = `/api/v1/cases/${caseId}/procedural-resources/${state.activities.resource.id}/activities/deadlines/prepare`;
  const preparations = [];
  let responseTask = Promise.resolve();
  state.handle = (route, call) => {
    if (call.path !== path) return false;
    expect(call.method).toBe('POST');
    preparations.push(structuredClone(call.body));
    responseTask = (async () => {
      const response = state.prepare(call.body);
      await new Promise((resolve) => setTimeout(resolve, 6000));
      await route.fulfill({ status: 200, json: response });
    })();
    return responseTask.then(() => true);
  };
  try {
    await page.goto('/');
    await page.evaluate(
      async ({ resource, caseId }) => {
        const { mountResourceDeadline } =
          await import('/tests/browser/resource-deadline-harness.mjs');
        await mountResourceDeadline(resource, caseId);
      },
      { resource: state.activities.resource, caseId },
    );
    await fillDeadline(page, 'Plazo contextual');
    await editor(page)
      .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
      .selectOption('follow');
    await prepareResourceDeadline(page);
    expect(preparations).toHaveLength(1);
    expect(state.submissions).toHaveLength(0);
    await expect(
      editor(page).getByRole('region', { name: 'Revision del plazo y vinculo', exact: true }),
    ).toBeVisible();
    await expect(
      editor(page).getByRole('button', { name: 'Confirmar plazo y vinculo', exact: true }),
    ).toBeEnabled();
    await expect(editor(page).getByRole('alert')).toHaveCount(0);
  } finally {
    // Keep the page alive until the single retained fixture response finishes.
    await responseTask;
  }
});
