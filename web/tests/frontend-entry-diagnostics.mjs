import { randomUUID } from 'node:crypto';
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const sourceRoot = fileURLToPath(new URL('../src/', import.meta.url));
const maximumCount = 1000000;

function publicSources() {
  const names = new Set();
  const pending = [{ directory: sourceRoot, prefix: '/src/' }];
  let visited = 0;
  while (pending.length && visited < 10000) {
    const { directory, prefix } = pending.pop();
    let entries;
    try {
      entries = readdirSync(directory, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (++visited > 10000) break;
      if (!/^[A-Za-z0-9_.-]+$/.test(entry.name)) continue;
      const name = prefix + entry.name;
      if (name.length > 240) continue;
      if (entry.isDirectory())
        pending.push({ directory: join(directory, entry.name), prefix: name + '/' });
      else if (entry.isFile() && /\.(?:svelte|mjs|js|css|astro)$/.test(entry.name)) names.add(name);
    }
  }
  return names;
}

function resourceName(value, origin, names) {
  try {
    const url = new URL(value, origin);
    if (url.origin !== origin) return 'other';
    if (names.has(url.pathname)) return url.pathname;
    if (url.pathname === '/') return 'document-root';
    if (url.pathname === '/@vite/client') return 'vite-client';
    if (url.pathname.startsWith('/node_modules/')) return 'dependency';
  } catch {
    // Unknown locations carry no public resource identity.
  }
  return 'other';
}

function browserObserver({ key, paths }) {
  const names = new Set(paths);
  const errors = [];
  let dropped = 0;
  const kinds = new Set(['Error', 'TypeError', 'SyntaxError', 'ReferenceError']);
  function componentName(value) {
    try {
      const url = new URL(value, location.href);
      if (url.origin === location.origin && names.has(url.pathname)) return url.pathname;
    } catch {
      // A malformed component URL is not copied into diagnostics.
    }
    return 'other';
  }
  function hydration(event) {
    try {
      if (errors.length === 4) {
        dropped = Math.min(dropped + 1, 1000000);
        return;
      }
      const name = event.detail?.error?.name;
      errors.push({
        component: componentName(event.detail?.componentUrl),
        error: kinds.has(name) ? name : 'other',
      });
    } catch {
      // Exceptions in third-party event properties are not diagnostic content.
    }
  }
  window.addEventListener('astro:hydration-error', hydration);
  Object.defineProperty(window, key, {
    configurable: true,
    value: {
      snapshot() {
        const elements = document.getElementsByTagName('astro-island');
        const islands = [];
        for (let index = 0; index < Math.min(elements.length, 4); index++) {
          const island = elements[index];
          const directive = island.getAttribute('client');
          islands.push({
            component: componentName(island.getAttribute('component-url')),
            client: ['only', 'load', 'idle', 'visible', 'media'].includes(directive)
              ? directive
              : 'other',
            ssr: island.hasAttribute('ssr'),
            children: Math.min(island.childElementCount, 1000),
          });
        }
        return {
          document: { available: true, readyState: document.readyState },
          islands,
          hydrationErrors: errors.map((value) => ({ ...value })),
          dropped: {
            hydrationErrors: dropped,
            islands: Math.min(Math.max(elements.length - 4, 0), 1000000),
          },
        };
      },
      dispose() {
        window.removeEventListener('astro:hydration-error', hydration);
      },
    },
  });
}

async function inspectPage(page, key, operation) {
  let timer;
  try {
    return await Promise.race([
      page
        .evaluate(({ key, operation }) => window[key]?.[operation](), { key, operation })
        .catch(() => null),
      new Promise((resolve) => {
        timer = setTimeout(() => resolve(null), 500);
      }),
    ]);
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

export async function observeFrontendEntry(page) {
  const names = publicSources();
  const key = `__frontend_entry_${randomUUID().replaceAll('-', '')}`;
  const failures = [];
  const recorded = new WeakSet();
  let origin = null;
  let dropped = 0;
  let disposed = false;
  let latest = null;
  function navigation(request) {
    if (disposed) return;
    try {
      if (request.isNavigationRequest() && request.frame() === page.mainFrame())
        origin = new URL(request.url()).origin;
    } catch {
      origin = null;
    }
  }
  function record(request, status) {
    if (disposed || recorded.has(request)) return;
    const type = request.resourceType();
    if (!['document', 'script', 'stylesheet'].includes(type)) return;
    recorded.add(request);
    if (failures.length === 12) {
      dropped = Math.min(dropped + 1, maximumCount);
      return;
    }
    failures.push({
      resource: resourceName(request.url(), origin, names),
      type,
      status,
      failure: status === null ? 'other' : null,
    });
  }
  const response = (value) => {
    if (value.status() >= 400 && value.status() <= 599) record(value.request(), value.status());
  };
  const failed = (request) => record(request, null);
  page.on('request', navigation);
  page.on('response', response);
  page.on('requestfailed', failed);
  try {
    await page.addInitScript(browserObserver, { key, paths: [...names] });
  } catch {
    // A closing page remains reportable without a browser-side observer.
  }

  async function snapshot() {
    if (disposed && latest) return structuredClone(latest);
    const browser = disposed ? null : await inspectPage(page, key, 'snapshot');
    latest = {
      version: 1,
      document: browser?.document ?? { available: false, readyState: 'unavailable' },
      islands: browser?.islands ?? [],
      failures: failures.map((value) => ({ ...value })),
      hydrationErrors: browser?.hydrationErrors ?? [],
      dropped: {
        failures: dropped,
        hydrationErrors: browser?.dropped.hydrationErrors ?? 0,
        islands: browser?.dropped.islands ?? 0,
      },
    };
    return structuredClone(latest);
  }

  async function attach(testInfo) {
    try {
      const value = await snapshot();
      let body = Buffer.from(JSON.stringify(value));
      if (body.length > 8192) {
        value.dropped.failures = Math.min(
          value.dropped.failures + value.failures.length,
          maximumCount,
        );
        value.failures = [];
        body = Buffer.from(JSON.stringify(value));
      }
      if (body.length > 8192) return false;
      await testInfo.attach('frontend-entry', { body, contentType: 'application/json' });
      return true;
    } catch {
      return false;
    }
  }

  async function dispose() {
    if (disposed) return;
    disposed = true;
    page.off('request', navigation);
    page.off('response', response);
    page.off('requestfailed', failed);
    await inspectPage(page, key, 'dispose');
  }
  return { snapshot, attach, dispose };
}
