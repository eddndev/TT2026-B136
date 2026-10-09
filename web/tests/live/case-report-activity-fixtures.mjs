import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { expect } from '@playwright/test';
import { hearingResult, measureDecision } from './case-report-activity-hearings.mjs';
import {
  calendarValues,
  declaredDate,
  profileDefinition,
  registration,
} from '../../../scripts/web-deadline-definitions.mjs';

const unknown = () => ({ kind: 'unknown', reason: 'Dato no declarado en soporte sintetico' });
const envelope = (prepared) => ({
  command: prepared.command,
  expected_submission_digest: prepared.submission_digest,
});

function api(token) {
  return async (method, path, body, expected = 200, headers = {}) => {
    const binary = Buffer.isBuffer(body);
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      redirect: 'error',
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(body === undefined
          ? {}
          : {
              'Content-Type': binary ? 'application/octet-stream' : 'application/json',
            }),
        ...headers,
      },
      body: body === undefined ? undefined : binary ? body : JSON.stringify(body),
    });
    expect(response.status, `${method} ${path}`).toBe(expected);
    return response.status === 204 ? null : response.json();
  };
}

async function session(account, recoveryIndex, run) {
  const anonymous = api();
  const challenge = await anonymous('POST', '/auth/login', {
    email: account.email,
    password: account.password,
  });
  const login = await anonymous('POST', '/auth/mfa/recovery', {
    challenge_token: challenge.challenge_token,
    code: account.recoveryCodes[recoveryIndex],
  });
  const call = api(login.access_token);
  try {
    expect((await call('GET', '/auth/me')).id).toBe(account.id);
    return await run(call);
  } finally {
    await call('POST', '/auth/logout', undefined, 204);
  }
}

async function persist(call, route, command, method = 'POST', suffix = '') {
  const prepared = await call('POST', `${route}/prepare`, command);
  const result = await call(method, route + suffix, envelope(prepared), 201);
  expect(result.receipt.operation_id).toBe(command.operation_id);
  expect(result.receipt.action).toBe(command.change.action);
  return result;
}

async function facts(call, path) {
  const vectors = JSON.parse(
    await readFile(
      new URL(
        '../../../crates/domain/tests/fixtures/procedural_fact_vectors.json',
        import.meta.url,
      ),
      'utf8',
    ),
  );
  const minimum = (family) =>
    structuredClone(vectors.find((row) => row.name === `${family}_minimum`).normalized);
  const values = minimum('resolution');
  values.summary = 'Resolucion original atribuida al litigante';
  values.issued_at = declaredDate();
  const resolution = await persist(call, `${path}/resolutions`, {
    operation_id: randomUUID(),
    id: randomUUID(),
    family: 'resolution',
    change: { action: 'record', expected_revision: 0, values },
  });
  const notice = minimum('notification');
  notice.resolution = { id: resolution.id, revision: resolution.revision };
  notice.summary = 'Notificacion original atribuida al litigante';
  const notification = await persist(call, `${path}/resolutions/${resolution.id}/notifications`, {
    operation_id: randomUUID(),
    id: randomUUID(),
    family: 'notification',
    resolution_id: resolution.id,
    change: { action: 'record', expected_revision: 0, values: notice },
  });
  return { resolution, notification };
}

async function resourceActs(call, path, resolution, support, author) {
  const route = `${path}/procedural-resources`;
  let resource = await persist(call, route, {
    operation_id: randomUUID(),
    resource_id: randomUUID(),
    change: {
      action: 'register',
      expected_revision: 0,
      values: {
        kind: 'appeal',
        mode: { kind: 'known', value: 'written' },
        title: 'Recurso para conteo de actuaciones originales',
        resolution: { id: resolution.id, revision: resolution.revision },
        resolution_evidence: support,
        resolution_reference: unknown(),
        issuing_authority: unknown(),
        receiving_authority: null,
        resolution_at: declaredDate(),
        notification_at: null,
        challenged_part: 'Apartado sintetico',
        grounds: 'Motivos declarados sin efecto inferido',
        appellants: [{ name: 'Persona recurrente sintetica', role: unknown(), participant: null }],
      },
    },
  });
  const acts = [];
  for (const kind of [
    'interposition',
    'admission',
    'inadmissibility',
    'withdrawal',
    'resolution',
  ]) {
    resource = await persist(
      call,
      route,
      {
        operation_id: randomUUID(),
        resource_id: resource.id,
        change: {
          action: 'record_act',
          expected_revision: resource.revision,
          act_id: randomUUID(),
          values: {
            kind,
            mode: { kind: 'known', value: 'written' },
            occurred_at: declaredDate(),
            authority: unknown(),
            statement: `Actuacion original ${kind}`,
            evidence: [support],
          },
        },
      },
      'POST',
      `/${resource.id}/acts`,
    );
    expect(resource.recorded_by.id).toBe(author.id);
    expect(resource.act.values.kind).toBe(kind);
    acts.push(resource.act);
  }
  return acts;
}

async function attendedDeadline(call, ownerCall, path, caseId, author, resolution) {
  const calendar = await persist(ownerCall, '/judicial-calendars', {
    operation_id: randomUUID(),
    calendar_id: randomUUID(),
    change: { action: 'publish', expected_revision: 0, values: calendarValues() },
  });
  const profile = await persist(ownerCall, `${path}/deadline-profiles`, {
    operation_id: randomUUID(),
    profile_id: randomUUID(),
    change: {
      action: 'publish',
      expected_revision: 0,
      definition: profileDefinition(caseId, 'daily', calendar.values),
    },
  });
  const route = `${path}/deadlines`;
  const deadline = await persist(
    call,
    route,
    registration(caseId, profile, resolution, calendar, author.id, 'Plazo atendido del informe'),
  );
  const command = {
    operation_id: randomUUID(),
    deadline_id: deadline.id,
    change: {
      action: 'set_attention',
      expected_revision: deadline.revision,
      reason: 'Atencion declarada por el litigante autor',
      attention: {
        status: 'recorded',
        occurred_at: { precision: 'unknown' },
        statement: 'Actuacion declarada sin certificar exito juridico',
        locator: 'Constancia sintetica',
      },
    },
  };
  const prepared = await call('POST', `${route}/prepare`, command);
  const payload = envelope(prepared);
  const attended = await call('POST', `${route}/${deadline.id}/attention`, payload, 201);
  expect(attended.receipt.operation_id).toBe(command.operation_id);
  expect(attended.receipt.action).toBe('set_attention');
  const repeated = await call('POST', `${route}/${deadline.id}/attention`, payload, 409);
  expect(repeated.error.code).toBe('deadline_operation_conflict');
  expect(await call('GET', `${route}/${deadline.id}/revisions/${attended.revision}`)).toEqual(
    attended,
  );
  return attended;
}

export async function provisionActivityCatalogue(owner) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL || !process.env.API_PROXY_TARGET)
    throw new Error('Activity catalogue requires disposable web-demo services');
  return session(owner, 7, async (ownerCall) => {
    const password = 'activity catalogue synthetic password';
    const row = await ownerCall(
      'POST',
      '/users',
      {
        email: `activity-catalogue-${randomUUID()}@example.com`,
        password,
        role: 'litigator',
      },
      201,
    );
    const author = {
      id: row.user.id,
      email: row.user.email,
      password,
      recoveryCodes: row.recovery_codes,
    };
    return session(author, 0, async (call) => {
      const reference = `ACTIVITY-${randomUUID().slice(0, 8)}`;
      const title = 'Actividad registrada del catalogo completo';
      const row = await call(
        'POST',
        '/penal-cases',
        {
          title,
          reference,
          profile: {
            nuc: `${reference}-NUC`,
            nuc_authority: 'Autoridad declarada',
            judicial_case_number: `${reference}-CJ`,
            judicial_authority: 'Organo declarado',
            offenses: ['Descripcion sintetica'],
            general_information: null,
            complementary_identifiers: null,
          },
        },
        201,
      );
      const path = `/cases/${row.id}`;
      const pdf = await readFile(
        new URL('../../../crates/infrastructure/tests/fixtures/stage-support.pdf', import.meta.url),
      );
      const document = await call('POST', `${path}/documents`, pdf, 201, {
        'X-Document-Name': 'activity-catalogue.pdf',
      });
      const support = {
        document_id: document.id,
        version: document.version,
        digest: document.digest,
      };
      const { resolution, notification } = await facts(call, path);
      const acts = await resourceActs(
        call,
        path,
        resolution,
        { ...support, locator: 'Pagina 1' },
        author,
      );
      const { hearing, result } = await hearingResult(call, path, persist);
      const decision = await measureDecision(call, path, row.id, hearing, support);
      const deadline = await attendedDeadline(call, ownerCall, path, row.id, author, resolution);
      for (const source of [resolution, notification, result]) {
        expect(source.recorded_by.id).toBe(author.id);
        expect(source.recorded_at).toEqual(expect.any(String));
      }
      expect(deadline.recorded_by.id).toBe(author.id);
      expect(deadline.recorded_at.unix_seconds).toEqual(expect.any(Number));
      expect(decision.group.review.actor.id).toBe(author.id);
      const now = Date.now();
      return {
        author,
        case: { id: row.id, title, reference },
        document,
        resolution,
        notification,
        acts,
        hearing,
        result,
        decision,
        deadline,
        from: new Date(now - 86400000).toISOString().slice(0, 10),
        before: new Date(now + 2 * 86400000).toISOString().slice(0, 10),
      };
    });
  });
}
