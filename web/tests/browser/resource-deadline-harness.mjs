import { mount } from 'svelte';
import { writable } from 'svelte/store';
import ResourceDeadlineEditor from '../../src/components/ResourceDeadlineEditor.svelte';
import { createApi } from '../../src/lib/api.mjs';

export async function mountResourceDeadline(resource, caseId) {
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
