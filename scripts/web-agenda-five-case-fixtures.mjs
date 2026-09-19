// Reuse an operator and exact calendar for independent cross-case agenda rows.
import { randomUUID } from "node:crypto";
import {
  declaredDate,
  profileDefinition,
  registration,
} from "./web-deadline-definitions.mjs";
import { persistFollowRecord } from "./web-deadline-reevaluation.mjs";

const date = "2026-03-06";
const scheduledAt = `${date}T23:30:00Z`;

async function hearing(call, record) {
  const route = `/cases/${record.id}/hearings`;
  const context = await call("GET", `${route}/context`);
  return persistFollowRecord(call, route, {
    operation_id: randomUUID(),
    hearing_id: randomUUID(),
    change: {
      action: "schedule",
      expected_revision: 0,
      expected_case_revision: context.case_revision,
      expected_stage_revision: context.stage_revision,
      values: {
        kind: "initial",
        scheduled_at: scheduledAt,
        modality: "in_person",
        venue: "Sede sintetica de consulta transversal",
        note: "Nota privada excluida del resumen transversal",
        participants: [],
        conviction_basis: null,
      },
    },
  });
}

async function deadline(call, record, operator, calendar) {
  const route = `/cases/${record.id}`;
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
        issued_at: { ...declaredDate(6), month: 3 },
        summary: `Fuente exacta ${record.reference}`,
        provenance: { kind: "operator_note", note: "Ejemplo sintetico" },
      },
    },
  });
  const profile = await persistFollowRecord(call, `${route}/deadline-profiles`, {
    profile_id: randomUUID(),
    operation_id: randomUUID(),
    change: {
      action: "publish",
      expected_revision: 0,
      definition: profileDefinition(record.id, "daily", calendar.values),
    },
  });
  const command = registration(
    record.id,
    profile,
    source,
    calendar,
    operator.id,
    `Plazo transversal ${record.reference}`,
  );
  command.change.tracking = {
    profile: "fixed",
    source: "fixed",
    calendar: "fixed",
  };
  const row = await persistFollowRecord(call, `${route}/deadlines`, command);
  const current = await call("GET", `${route}/deadlines/${row.id}`);
  const dueAt = current.operational.due_at;
  if (
    !dueAt || dueAt.unix_seconds !== Date.parse(scheduledAt) / 1000 ||
    dueAt.nanosecond !== 0 || dueAt.offset_seconds !== 0
  ) throw new Error("Cross-case deadline must match the declared UTC agenda day");
  return { deadline: row, dueAt };
}

export async function provisionFiveCaseAgenda(call, operator, calendar) {
  const visible = [];
  let hidden;
  for (let index = 0; index < 6; index += 1) {
    const reference = `AGENDA-CROSS-${index + 1}`;
    const title = `Agenda transversal ${index + 1}`;
    const created = await call("POST", "/penal-cases", {
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
    }, 201);
    const record = { id: created.id, title, reference };
    if (index < 5)
      await call("PUT", `/cases/${record.id}/members/${operator.id}`, undefined, 204);
    const kind = index < 3 || index === 5 ? "hearing" : "deadline";
    const entry = { case: record, kind };
    if (kind === "hearing") entry.hearing = await hearing(call, record);
    else Object.assign(entry, await deadline(call, record, operator, calendar));
    if (index === 5) hidden = entry;
    else visible.push(entry);
  }
  return { operator, date, visible, hidden };
}
