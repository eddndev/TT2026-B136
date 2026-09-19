// Independent Follow scenarios use public API commands and synthetic arithmetic.
import { randomUUID } from "node:crypto";
import { setTimeout as delay } from "node:timers/promises";
import {
  calendarValues,
  declaredDate,
  profileDefinition,
  registration,
} from "./web-deadline-definitions.mjs";

export async function persistFollowRecord(
  call,
  route,
  command,
  method = "POST",
  suffix = "",
) {
  const draft = await call("POST", `${route}/prepare`, command);
  return call(
    method,
    route + suffix,
    {
      command: draft.command,
      expected_submission_digest: draft.submission_digest,
    },
    201,
  );
}

async function account(call, label, role) {
  const password = `deadline follow ${label} ${role} password`;
  const row = await call(
    "POST",
    "/users",
    {
      email: `deadline-follow.${label}.${role}@example.com`,
      password,
      role,
    },
    201,
  );
  return {
    id: row.user.id,
    email: row.user.email,
    password,
    recoveryCodes: row.recovery_codes,
  };
}

async function scenario(call, label) {
  const owner = await account(call, label, "owner");
  const operator = await account(call, label, "litigator");
  const reference = `DEADLINE-FOLLOW-${label.toUpperCase()}`;
  const title = `Reevaluacion de plazos ${label}`;
  const created = await call(
    "POST",
    "/penal-cases",
    {
      title,
      reference,
      profile: {
        nuc: `${reference}-NUC`,
        nuc_authority: "Autoridad sintetica",
        judicial_case_number: `${reference}-CJ`,
        judicial_authority: "Organo sintetico",
        offenses: ["Descripcion sintetica"],
        general_information: null,
        complementary_identifiers: null,
      },
    },
    201,
  );
  const record = { id: created.id, title, reference };
  const route = `/cases/${record.id}`;
  await call("PUT", `${route}/members/${operator.id}`, undefined, 204);
  const source = await persistFollowRecord(call, `${route}/resolutions`, {
    family: "resolution",
    id: randomUUID(),
    operation_id: randomUUID(),
    change: {
      action: "record",
      expected_revision: 0,
      values: {
        subtype: null,
        class: { kind: "known", value: { kind: "order" } },
        issuer: { kind: "known", value: "Autoridad sintetica" },
        issued_at: declaredDate(31),
        summary: `Fuente Follow ${label}`,
        provenance: { kind: "operator_note", note: "Ejemplo sintetico" },
      },
    },
  });
  const calendarDefinition = calendarValues();
  calendarDefinition.scope.title = `Calendario sintetico Follow ${label}`;
  const calendar = await persistFollowRecord(call, "/judicial-calendars", {
    calendar_id: randomUUID(),
    operation_id: randomUUID(),
    change: {
      action: "publish",
      expected_revision: 0,
      values: calendarDefinition,
    },
  });
  const profile = await persistFollowRecord(
    call,
    `${route}/deadline-profiles`,
    {
      profile_id: randomUUID(),
      operation_id: randomUUID(),
      change: {
        action: "publish",
        expected_revision: 0,
        definition: profileDefinition(record.id, "daily", calendar.values),
      },
    },
  );
  const command = registration(
    record.id,
    profile,
    source,
    calendar,
    operator.id,
    `Seguimiento Follow ${label}`,
  );
  command.change.tracking = {
    profile: "follow",
    source: "follow",
    calendar: "follow",
  };
  const deadline = await persistFollowRecord(
    call,
    `${route}/deadlines`,
    command,
  );
  return { case: record, owner, operator, source, calendar, profile, deadline };
}

export async function provisionDeadlineReevaluation(call, prefix = "") {
  if (process.env.TT_DEADLINE_REEVALUATION_ACCEPTANCE !== "1")
    throw new Error(
      "Explicit deadline reevaluation acceptance opt-in is required",
    );
  if (!process.env.IDENTITY_TEST_DATABASE_URL)
    throw new Error("Disposable browser backends are required");
  const desktop = await scenario(call, `${prefix}desktop`);
  const mobile = await scenario(call, `${prefix}mobile`);
  return { desktop, mobile };
}

export async function advanceFollowCalendar(call, calendar) {
  const values = structuredClone(calendar.values);
  values.exceptions.push({
    id: randomUUID(),
    from: "2026-01-31",
    through: "2026-01-31",
    classification: "excluded",
    source_ids: [values.sources[0].id],
    explanation: "Dia excluido expresamente en el ejemplo",
  });
  return persistFollowRecord(
    call,
    "/judicial-calendars",
    {
      calendar_id: calendar.id,
      operation_id: randomUUID(),
      change: {
        action: "replace",
        expected_revision: calendar.revision,
        values,
        reason: "Revisar los dias contables del ejemplo",
      },
    },
    "PUT",
    `/${calendar.id}`,
  );
}

export async function advanceFollowSource(call, caseId, source) {
  const values = structuredClone(source.values);
  values.issued_at = {
    precision: "date",
    year: 2026,
    month: 2,
    day: 2,
    offset_seconds: null,
  };
  values.summary = "Fuente Follow con fecha corregida expresamente";
  return persistFollowRecord(
    call,
    `/cases/${caseId}/resolutions`,
    {
      family: "resolution",
      id: source.id,
      operation_id: randomUUID(),
      change: {
        action: "correct",
        expected_revision: source.revision,
        values,
        reason: "Cambiar la fecha declarada del ejemplo",
      },
    },
    "PUT",
    `/${source.id}`,
  );
}

// The caller supplies bounded HTTP requests; one request may outlast the polling deadline.
export async function waitForDeadlineRevision(
  call,
  caseId,
  id,
  expected,
  timeoutMs = 60000,
) {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    const value = await call("GET", `/cases/${caseId}/deadlines/${id}`);
    if (value.revision > expected)
      throw new Error("Unexpected additional deadline revision");
    if (value.revision === expected) return value;
    await delay(Math.min(250, Math.max(0, until - Date.now())));
  }
  throw new Error(
    `Deadline did not reach revision ${expected} within ${timeoutMs} ms`,
  );
}
