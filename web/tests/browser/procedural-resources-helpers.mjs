import { createHash } from 'node:crypto';
import { expect } from '@playwright/test';
import { setupFacts, failFact } from './procedural-facts-helpers.mjs';
import { login, navigate, caseId, document } from './helpers.mjs';
import {
  factRecord,
  factResolutionSource,
  factAdministration,
  resolutionId,
} from '../fixtures/procedural-facts.mjs';
import {
  resourceValuesFixture,
  resourceActor,
  resourceRecord,
  resourcePrepared,
} from '../fixtures/procedural-resource-unit.mjs';
export const resourceEditor = (page) =>
  page.getByRole('region', { name: 'Formulario de recurso', exact: true });
export const resourceDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de recurso', exact: true });
export function browserResource() {
  const draft = resourcePrepared(),
    values = resourceValuesFixture();
  values.resolution = { id: resolutionId, revision: 1 };
  values.resolution_evidence = {
    document_id: document.id,
    version: document.version,
    digest: document.digest,
    locator: 'Pagina 1',
  };
  Object.assign(draft, {
    case_id: caseId,
    values,
    observed_administration: structuredClone(factAdministration),
    observed_stage: { case_id: caseId, current: null },
  });
  draft.command.change.values = structuredClone(values);
  draft.sources = {
    resolution: factResolutionSource(factRecord()),
    appellants: [],
    supports: [
      {
        document_id: document.id,
        version: document.version,
        digest: document.digest,
        name: document.name,
        format: 'pdf',
        policy: 'pdf_docx_v1',
      },
    ],
  };
  return resourceRecord(draft);
}
export async function setupProceduralResources(page, { resources = [], ...options } = {}) {
  const facts = await setupFacts(page, { ...options, facts: options.facts || [factRecord()] });
  const role = options.role || 'owner',
    user = { ...resourceActor, role };
  await page.route(/\/api\/v1\/auth\/(me|mfa\/.*)$/, (route) => {
    const path = new URL(route.request().url()).pathname;
    return route.fulfill({
      json: path.includes('/mfa/')
        ? { access_token: 'resource-session', user, expires_in_seconds: 86400 }
        : user,
    });
  });
  const state = { facts, calls: [], submissions: [], records: new Map(), handle: null };
  for (const row of resources)
    state.records.set(row.id, [...(state.records.get(row.id) || []), structuredClone(row)]);
  state.prepare = (command) => {
    const history = state.records.get(command.resource_id) || [],
      base = history.at(-1);
    const change = command.change,
      draft = resourcePrepared(command);
    const values = ['register', 'correct'].includes(change.action)
      ? structuredClone(change.values)
      : structuredClone(base.values);
    Object.assign(draft, {
      case_id: caseId,
      values,
      recorded_by: { ...resourceActor },
      observed_administration: structuredClone(factAdministration),
      observed_stage: { case_id: caseId, current: null },
      previous: base
        ? { revision: base.revision, capture_digest: base.receipt.capture_digest }
        : null,
    });
    const parent = [...facts.records.values()]
      .flat()
      .find((v) => v.id === values.resolution.id && v.revision === values.resolution.revision);
    if (!parent) throw new Error('Missing exact resolution fixture');
    const captures = (refs) =>
      [...new Map(refs.map((v) => [`${v.document_id}/${v.version}`, v])).values()]
        .sort((a, b) => a.document_id.localeCompare(b.document_id) || a.version - b.version)
        .map(({ locator, ...ref }) => ({
          ...ref,
          name: document.name,
          format: 'pdf',
          policy: 'pdf_docx_v1',
        }));
    draft.sources = {
      resolution: factResolutionSource(parent),
      supports: captures([values.resolution_evidence]),
      appellants: values.appellants
        .map((v) => v.participant)
        .filter(Boolean)
        .sort((a, b) => a.id.localeCompare(b.id) || a.revision - b.revision)
        .map((ref) => {
          const row = facts.results.directory.get(ref.id)?.find((v) => v.revision === ref.revision);
          if (!row) throw new Error('Missing exact participant fixture');
          return {
            case_id: caseId,
            ...ref,
            values_digest: row.values_digest,
            directory_status: row.directory_status,
            subject: row.subject
              ? {
                  id: row.subject.id,
                  revision: row.subject.revision,
                  values_digest: row.subject.values_digest,
                }
              : null,
            display_name: row.display_name,
            procedural_role: row.procedural_role,
            organization: row.organization || null,
            kind: row.profile?.kind || null,
          };
        }),
    };
    if (draft.act) {
      draft.act.revision = (change.expected_act_revision || 0) + 1;
      draft.act.supports = captures(change.values.evidence);
      const previousAct = history.findLast((v) => v.act?.id === change.act_id);
      draft.act.previous =
        change.action === 'correct_act'
          ? { revision: previousAct.revision, capture_digest: previousAct.receipt.capture_digest }
          : null;
    }
    draft.submission_digest = createHash('sha256')
      .update(JSON.stringify({ command, values, previous: draft.previous }))
      .digest('hex');
    return draft;
  };
  state.commit = (draft) => {
    const row = resourceRecord(draft);
    row.receipt.capture_digest = createHash('sha256').update(JSON.stringify(row)).digest('hex');
    state.records.set(row.id, [...(state.records.get(row.id) || []), row]);
    return row;
  };
  await page.route('**/api/v1/cases/*/procedural-resources**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const parts = url.pathname.split('/procedural-resources')[1].split('/').filter(Boolean);
    const call = {
      path: url.pathname,
      method: request.method(),
      body: request.postDataJSON(),
      search: url.search,
    };
    state.calls.push(call);
    if (facts.results.scheduling.denied || role === 'client')
      return failFact(route, 'permission_denied', 403);
    if (state.handle && (await state.handle(route, call))) return;
    if (call.method !== 'GET') {
      if (role === 'paralegal') return failFact(route, 'permission_denied', 403);
      if (facts.results.scheduling.context.administrative_status === 'closed')
        return failFact(route, 'case_closed');
      const command = parts[0] === 'prepare' ? call.body : call.body.command;
      const current = state.records.get(command.resource_id)?.at(-1);
      if ((current?.revision || 0) !== command.change.expected_revision)
        return failFact(route, 'procedural_resource_revision_conflict');
      const draft = state.prepare(command);
      if (parts[0] === 'prepare') return route.fulfill({ json: draft });
      state.submissions.push(command);
      if (draft.submission_digest !== call.body.expected_submission_digest)
        return failFact(route, 'procedural_resource_submission_mismatch');
      return route.fulfill({ status: 201, json: state.commit(draft) });
    }
    if (!parts.length) {
      let rows = [...state.records.values()]
        .map((v) => v.at(-1))
        .filter(
          (row) =>
            (!url.searchParams.has('kind') || row.values.kind === url.searchParams.get('kind')) &&
            (!url.searchParams.has('status') || row.status === url.searchParams.get('status')) &&
            (!url.searchParams.has('after_id') || row.id > url.searchParams.get('after_id')),
        )
        .sort((a, b) => a.id.localeCompare(b.id));
      const limit = Number(url.searchParams.get('limit')),
        more = rows.length > limit;
      rows = rows.slice(0, limit);
      return route.fulfill({
        json: { resources: rows, has_more: more, next_after_id: more ? rows.at(-1).id : null },
      });
    }
    const rows = state.records.get(parts[0]);
    if (!rows) return failFact(route, 'procedural_resource_not_found', 404);
    if (parts[1] === 'history') {
      let selected = [...rows]
        .reverse()
        .filter(
          (row) =>
            !url.searchParams.has('before_revision') ||
            row.revision < Number(url.searchParams.get('before_revision')),
        );
      const limit = Number(url.searchParams.get('limit')),
        more = selected.length > limit;
      selected = selected.slice(0, limit);
      return route.fulfill({
        json: {
          revisions: selected,
          has_more: more,
          next_before_revision: more ? selected.at(-1).revision : null,
        },
      });
    }
    const row =
      parts[1] === 'revisions' ? rows.find((v) => v.revision === Number(parts[2])) : rows.at(-1);
    return row
      ? route.fulfill({ json: row })
      : failFact(route, 'procedural_resource_not_found', 404);
  });
  return state;
}
export async function openResources(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Recursos', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Recursos procesales', exact: true }),
  ).toBeVisible();
}
