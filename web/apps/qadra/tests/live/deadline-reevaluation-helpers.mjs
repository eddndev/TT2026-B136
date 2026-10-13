import { expect } from '@playwright/test';
import { detail, list, openDeadline } from './deadline-helpers.mjs';

export const following = (page) =>
  detail(page).getByRole('region', {
    name: 'Seguimiento del plazo',
    exact: true,
  });
export const calculation = (page) =>
  detail(page).getByRole('region', {
    name: 'Resultado del plazo',
    exact: true,
  });
export const dueAt = (iso) => ({
  unix_seconds: Date.parse(iso) / 1000,
  nanosecond: 0,
  offset_seconds: 0,
});
export function historical(row) {
  return {
    ...structuredClone(row),
    operational: {
      freshness: 'not_checked',
      checked_at: null,
      changed_dependencies: [],
      due_at: null,
    },
  };
}

export async function withReevaluationApi(account, action) {
  let token;
  async function call(method, path, data, expected = 200) {
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      signal: AbortSignal.timeout(5000),
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(data === undefined ? {} : { 'Content-Type': 'application/json' }),
      },
      body: data === undefined ? undefined : JSON.stringify(data),
    });
    expect(response.status, `${method} ${path}`).toBe(expected);
    return response.status === 204 ? null : response.json();
  }
  const challenge = await call('POST', '/auth/login', {
    email: account.email,
    password: account.password,
  });
  token = (
    await call('POST', '/auth/mfa/recovery', {
      challenge_token: challenge.challenge_token,
      code: account.recoveryCodes[0],
    })
  ).access_token;
  try {
    return await action(call);
  } finally {
    await call('POST', '/auth/logout', undefined, 204);
  }
}

export async function refreshDeadline(page, row) {
  await page.getByRole('button', { name: 'Actualizar plazos', exact: true }).click();
  await expect(list(page)).toHaveAttribute('aria-busy', 'false');
  await openDeadline(page, row);
}

export function technicalReceipt(row, predecessor, dependency, family, review) {
  expect(row.receipt.action).toBe('reevaluate');
  expect(row.receipt.version.kind).toBe('v2');
  expect(row.recorded_by).toEqual({
    kind: 'technical',
    service: 'deadline_reevaluator',
    policy_version: 1,
  });
  expect(row.receipt.version.predecessor).toEqual({
    submission_digest: predecessor.receipt.submission_digest,
    capture_digest: predecessor.receipt.capture_digest,
  });
  expect(row.receipt.version.cause).toMatchObject({
    kind: 'source_event',
    event: {
      family,
      source_id: dependency.id,
      revision: dependency.revision,
      operation_id: dependency.receipt.operation_id,
    },
  });
  expect(row.receipt.version.cause.event.sequence).toMatch(/^[1-9][0-9]*$/);
  expect(row.tracking.review.state).toBe(review);
}

export async function captureReevaluation(page, testInfo, name) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
}
