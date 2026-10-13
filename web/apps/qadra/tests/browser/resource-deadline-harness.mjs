import { mount, tick } from 'svelte';
import { writable } from 'svelte/store';
import ResourceDeadlineEditor from '../../src/components/ResourceDeadlineEditor.svelte';
import { createApi } from '../../src/lib/api.mjs';

export async function mountResourceDeadline(resource, caseId) {
  const app = document.querySelector('astro-island[component-url*="/App.svelte"]');
  if (!app) throw new Error('The deadline harness requires the application island');
  if (app.hasAttribute('ssr')) {
    await new Promise((resolve) => app.addEventListener('astro:hydrate', resolve, { once: true }));
  }
  // Finish the login mount effects before the harness can receive keyboard input.
  await tick();
  const api = createApi();
  const { user } = await api.mfa('challenge', 'recovery-code', 'recovery');
  const target = document.createElement('section');
  target.id = 'resource-deadline-harness';
  document.body.appendChild(target);
  const state = writable({ closed: false });
  mount(ResourceDeadlineEditor, {
    target,
    context: new Map([['case-administration', state]]),
    props: {
      api,
      caseId,
      user,
      resource,
      head: resource,
      ondenied: () => {
        target.dataset.denied = 'true';
      },
      oncancel: () => {},
      onconfirmed: () => {
        target.dataset.confirmed = 'true';
      },
    },
  });
  return () => state.set({ closed: true });
}
