// Provision synthetic compound-result arithmetic through the disposable API.
import { randomUUID } from "node:crypto";
import { profileDefinition } from "./web-deadline-definitions.mjs";

async function persist(call, route, command) {
  const prepared = await call("POST", `${route}/prepare`, command);
  return call(
    "POST",
    route,
    {
      command: prepared.command,
      expected_submission_digest: prepared.submission_digest,
    },
    201,
  );
}

export async function provisionHearingDerivedDeadlines(call) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL)
    throw new Error("disposable browser services are required");
  const password = "hearing derived deadline owner password";
  const account = await call(
    "POST",
    "/users",
    {
      email: "hearing-derived-deadline.owner@example.com",
      password,
      role: "owner",
    },
    201,
  );
  const owner = {
    id: account.user.id,
    email: account.user.email,
    password,
    recoveryCodes: account.recovery_codes,
  };
  const title = "Resultado y plazo conjunto reproducible";
  const reference = "HEARING-DERIVED-WORKFLOW";
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
  const path = `/cases/${record.id}`;
  const context = await call("GET", `${path}/hearings/context`);
  const seconds = Math.floor(Date.now() / 1000) - 12 * 3600;
  const local = new Date((seconds - 6 * 3600) * 1000).toISOString();
  const instant = {
    date: local.slice(0, 10),
    time: local.slice(11, 19),
    offset: "-06:00",
  };
  instant.at = `${instant.date}T${instant.time}${instant.offset}`;
  const hearing = await persist(call, `${path}/hearings`, {
    operation_id: randomUUID(),
    hearing_id: randomUUID(),
    change: {
      action: "schedule",
      expected_revision: 0,
      expected_case_revision: context.case_revision,
      expected_stage_revision: context.stage_revision,
      values: {
        kind: "initial",
        scheduled_at: instant.at,
        modality: "in_person",
        venue: "Sede sintetica del resultado",
        note: null,
        participants: [],
        conviction_basis: null,
      },
    },
  });
  const definition = profileDefinition(record.id, "hourly", null);
  definition.title = "Horas desde el hecho declarado";
  definition.trigger = {
    kind: "source_field",
    field: "hearing_session_event_time",
  };
  definition.template = {
    kind: "fixed",
    rule: { kind: "elapsed_hours", quantity: 24 },
  };
  definition.examples[0].ordered_quantity = null;
  const profile = await persist(call, `${path}/deadline-profiles`, {
    operation_id: randomUUID(),
    profile_id: randomUUID(),
    change: { action: "publish", expected_revision: 0, definition },
  });
  return {
    owner,
    case: record,
    hearing,
    profile,
    instant,
    dueAt: {
      unix_seconds: seconds + 24 * 3600,
      nanosecond: 0,
      offset_seconds: 0,
    },
  };
}
