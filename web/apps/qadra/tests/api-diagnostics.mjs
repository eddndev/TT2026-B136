import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

export async function apiFailure(response) {
  const status = response.status();
  const pathname = new URL(response.url()).pathname;
  if (status < 400 || !pathname.startsWith('/api/v1/')) return null;
  const payload = await response.json().catch(() => ({}));
  const code = payload?.error?.code;
  return {
    method: response.request().method(),
    pathname: pathname.replace(/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}/gi, ':id'),
    status,
    code: typeof code === 'string' && /^[a-z][a-z0-9_]{0,79}$/.test(code) ? code : 'unknown',
  };
}

const reports = new Map();
export function watchApiFailures(page, testInfo) {
  const path = testInfo.outputPath('api-failures.json');
  if (!reports.has(path)) reports.set(path, { pages: new WeakSet(), failures: [] });
  const report = reports.get(path);
  if (report.pages.has(page)) return;
  report.pages.add(page);
  page.on('response', async (response) => {
    const failure = await apiFailure(response);
    if (!failure) return;
    report.failures.push(failure);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, `${JSON.stringify(report.failures, null, 2)}\n`);
  });
}
