import { test, expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import {
  setupResourceDeadline,
  fillDeadline,
  editor,
  prepareResourceDeadline,
  submitResourceDeadline,
} from './resource-deadline-helpers.mjs';

for (const paired of [true, false])
  test(`administrative closure ${paired ? 'allows joint-origin confirmation' : 'blocks absent-record retry'}`, async ({
    page,
  }) => {
    const state = await setupResourceDeadline(page);
    state.handle = async (route, call) => {
      if (!call.path.endsWith('/submit') || state.submissions.length) return false;
      state.submissions.push(structuredClone(call.body));
      if (paired) state.commit(state.prepare(call.body.command));
      await route.abort('failed');
      return true;
    };
    await page.goto('/');
    await page.evaluate(
      async ({ resource, caseId }) => {
        const { mountResourceDeadline } =
          await import('/tests/browser/resource-deadline-harness.mjs');
        window.closeContextualDeadlineCase = await mountResourceDeadline(resource, caseId);
      },
      { resource: state.activities.resource, caseId },
    );
    await fillDeadline(page, 'Plazo contextual');
    await editor(page)
      .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
      .selectOption('follow');
    await prepareResourceDeadline(page);
    await submitResourceDeadline(page);
    await expect(
      editor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
    ).toBeVisible();
    await page.evaluate(() => window.closeContextualDeadlineCase());
    await expect(editor(page)).toContainText(
      'El expediente est\u00e1 cerrado administrativamente. Tu formulario se conserva.',
    );
    await editor(page).getByRole('button', { name: 'Consultar resultado', exact: true }).click();
    expect(state.submissions).toHaveLength(1);
    const button = editor(page).getByRole('button', {
      name: paired ? 'Confirmar origen del env\u00edo' : 'Reintentar envio exacto',
      exact: true,
    });
    await expect(button).toBeVisible();
    if (paired) {
      await expect(button).toBeEnabled();
      await button.click();
      await expect(page.locator('#resource-deadline-harness')).toHaveAttribute(
        'data-confirmed',
        'true',
      );
      expect(state.submissions).toHaveLength(2);
      expect(state.submissions[1]).toEqual(state.submissions[0]);
    } else {
      await expect(button).toBeDisabled();
      await expect(page.locator('#resource-deadline-harness')).not.toHaveAttribute(
        'data-confirmed',
      );
      expect(state.submissions).toHaveLength(1);
    }
  });
