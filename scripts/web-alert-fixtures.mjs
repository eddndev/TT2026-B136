// Reuse hearing staff accounts for independent personal alert acceptance.
import { randomUUID } from "node:crypto";

export async function provisionAlerts(call, accounts) {
  const row = await call("POST", "/penal-cases", {
    title: "Alertas personales de audiencia",
    reference: "ALERT-BROWSER",
    profile: {
      nuc: "ALERT-BROWSER-NUC",
      nuc_authority: "Autoridad declarada",
      judicial_case_number: "ALERT-BROWSER-CJ",
      judicial_authority: "Organo declarado",
      offenses: ["Descripcion sintetica"],
      general_information: null,
      complementary_identifiers: null,
    },
  }, 201);
  for (const role of ["litigator", "paralegal"])
    await call("PUT", `/cases/${row.id}/members/${accounts[role].id}`, undefined, 204);
  const route = `/cases/${row.id}/hearings`;
  const context = await call("GET", `${route}/context`);
  const scheduledAt = new Date(Date.now() + 36 * 3600000).toISOString().replace(/\.\d{3}Z$/, "Z");
  const command = {
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
        venue: "Sede sintetica de alertas",
        note: null,
        participants: [],
        conviction_basis: null,
      },
    },
  };
  const prepared = await call("POST", `${route}/prepare`, command);
  const hearing = await call("POST", route, {
    command: prepared.command,
    expected_submission_digest: prepared.submission_digest,
  }, 201);
  return {
    case: { id: row.id, title: row.administration.title, reference: row.administration.reference },
    hearing,
    desktop: accounts.litigator,
    mobile: accounts.paralegal,
  };
}
