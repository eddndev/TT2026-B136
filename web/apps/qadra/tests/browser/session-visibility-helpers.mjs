import { expect } from '@playwright/test';
import { checkSessionRequests } from './session-inactivity-helpers.mjs';

const controls = new WeakMap();

export async function visibility(page, state) {
  await page.evaluate((value) => {
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      get: () => value,
    });
    Object.defineProperty(document, 'hidden', {
      configurable: true,
      get: () => value === 'hidden',
    });
    document.dispatchEvent(new Event('visibilitychange'));
  }, state);
}

export async function controlRoute(page, path, token, handle) {
  const control = { path, token, calls: [], release: () => {} };
  const owned = controls.get(page) || [];
  owned.push(control);
  controls.set(page, owned);
  await page.route(
    (url) => url.pathname === path,
    async (route) => {
      const request = route.request();
      control.calls.push({
        method: request.method(),
        url: request.url(),
        body: request.postData(),
        headers: request.headers(),
      });
      return handle(route, control.calls.length);
    },
  );
  return control;
}

export async function holdControl(page, path, token, failure = false) {
  let release;
  const pending = new Promise((resolve) => {
    release = resolve;
  });
  const control = await controlRoute(page, path, token, async (route, count) => {
    if (count === 1) await pending;
    return failure ? route.abort('internetdisconnected') : route.fallback();
  });
  control.release = release;
  return control;
}

export async function finishVisibilityRequests(page) {
  for (const control of controls.get(page) || []) {
    control.release();
    for (const call of control.calls) {
      expect(call.method).toBe(control.path.endsWith('/logout') ? 'POST' : 'GET');
      expect(call.body).toBeNull();
      expect(call.url).not.toContain('?');
      expect(call.headers.authorization).toBe(`Bearer ${control.token}`);
      expect(call.headers['content-type']).toBeUndefined();
    }
  }
  await checkSessionRequests(page);
}
