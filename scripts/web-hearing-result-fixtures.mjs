// Provision result scenarios through the caller's disposable authenticated API.
import { randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";

export async function provisionHearingResults(call) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL)
    throw new Error("disposable browser services are required");
  const result = {};
  for (const role of ["owner", "litigator", "paralegal", "client"]) {
    const password = `hearing results browser ${role} password`;
    const row = await call(
      "POST",
      "/users",
      {
        email: `hearing-results.${role}@example.com`,
        password,
        role,
      },
      201,
    );
    result[role] = {
      id: row.user.id,
      email: row.user.email,
      password,
      recoveryCodes: row.recovery_codes,
    };
  }
  for (const [key, title, reference] of [
    ["case", "Sesiones con evidencia historica", "RESULT-WORKFLOW"],
    ["policyCase", "Permisos de resultados declarados", "RESULT-POLICY"],
    ["raceCase", "Rectificaciones concurrentes de resultados", "RESULT-RACE"],
  ]) {
    const row = await call(
      "POST",
      "/penal-cases",
      {
        title,
        reference,
        profile: {
          nuc: `${reference}-NUC`,
          nuc_authority: "Autoridad declarada",
          judicial_case_number: `${reference}-CJ`,
          judicial_authority: "Organo declarado",
          offenses: ["Descripcion sintetica"],
          general_information: null,
          complementary_identifiers: null,
        },
      },
      201,
    );
    result[key] = { id: row.id, title, reference };
    for (const role of ["litigator", "paralegal", "client"])
      await call(
        "PUT",
        `/cases/${row.id}/members/${result[role].id}`,
        undefined,
        204,
      );
  }
  const path = `/cases/${result.case.id}`;
  result.manual = await call(
    "POST",
    `${path}/participants`,
    {
      display_name: "Compareciente historico",
      procedural_role: "Testigo",
    },
    201,
  );
  await call("PUT", `${path}/participants/${result.manual.id}`, {
    expected_revision: 1,
    display_name: "Nombre posterior archivado",
    procedural_role: "Testigo",
    organization: null,
    legal_status: null,
    directory_status: "active",
  });
  await call(
    "PUT",
    `${path}/participants/${result.manual.id}/directory-status`,
    {
      expected_revision: 2,
      directory_status: "archived",
    },
  );
  const pdf = await readFile(
    new URL(
      "../crates/infrastructure/tests/fixtures/stage-support.pdf",
      import.meta.url,
    ),
  );
  const document = await call("POST", `${path}/documents`, pdf, 201, {
    "X-Document-Name": "result-support.pdf",
  });
  result.support = {
    document_id: document.id,
    version: 1,
    digest: document.digest,
  };
  const locator = { ...result.support, locator: "Pagina 1 sintetica" };
  const proposal = await call("POST", `${path}/participants/proposals/review`, {
    subject: {
      operation: "create",
      values: {
        kind: "natural_person",
        name: { state: "known", value: "Identidad historica declarada" },
        curp: { state: "unknown", reason: "Dato no declarado" },
        identity_support: locator,
      },
    },
    participant: { operation: "create" },
    role: {
      organization: null,
      legal_status: null,
      profile: {
        kind: "defendant",
        custody: { state: "unknown", reason: "Dato no declarado" },
      },
      role_support: locator,
    },
    certificate_base64: null,
  });
  const prepared = {
    proposal: proposal.proposal,
    review: {
      directory_stamp: proposal.directory_stamp,
      selection_reason: "Identidad sintetica revisada",
      different: [],
    },
    certificate_base64: null,
  };
  await call("POST", `${path}/participants/proposals/prepare`, prepared);
  result.typed = await call(
    "POST",
    `${path}/participants/proposals/commit`,
    {
      prepared,
      signature_base64: null,
    },
    201,
  );
  const subjectPath = `${path}/subjects/${result.typed.subject.id}`;
  const values = structuredClone(result.typed.subject.values);
  values.name.value = "Identidad actual distinta";
  const review = await call("POST", `${subjectPath}/review`, {
    expected_revision: 1,
    values,
  });
  await call("PUT", subjectPath, {
    expected_revision: 1,
    values,
    review: {
      directory_stamp: review.directory_stamp,
      selection_reason: "Correccion declarada",
      different: [],
    },
  });
  await call(
    "PUT",
    `${path}/participants/${result.typed.id}/directory-status`,
    {
      expected_revision: 1,
      directory_status: "archived",
    },
  );
  await call("POST", `${path}/documents/${document.id}/versions/1/seal`);
  await call(
    "POST",
    `${path}/documents/${document.id}/versions?expected_version=1`,
    pdf,
    201,
    {
      "X-Document-Name": "result-support-later.pdf",
    },
  );
  result.hearing = await schedule(call, result.case.id);
  result.nextHearing = await schedule(call, result.case.id);
  result.policyHearing = await schedule(call, result.policyCase.id);
  result.raceHearing = await schedule(call, result.raceCase.id);
  result.originalHearing = structuredClone(result.hearing);
  result.hearing = await changeHearing(
    call,
    result.case.id,
    result.hearing,
    "replace",
  );
  result.hearing = await changeHearing(
    call,
    result.case.id,
    result.hearing,
    "cancel",
  );
  result.nextHearing = await changeHearing(
    call,
    result.case.id,
    result.nextHearing,
    "cancel",
  );
  result.policyResult = await seedResult(
    call,
    result.policyCase.id,
    result.policyHearing,
  );
  result.raceResult = await seedResult(
    call,
    result.raceCase.id,
    result.raceHearing,
  );
  return result;
}
async function hearingCommand(call, caseId, command) {
  const path = `/cases/${caseId}/hearings`;
  const prepared = await call("POST", `${path}/prepare`, command);
  const action = command.change.action;
  return call(
    action === "replace" ? "PUT" : "POST",
    action === "schedule"
      ? path
      : `${path}/${command.hearing_id}${action === "cancel" ? "/cancellation" : ""}`,
    {
      command: prepared.command,
      expected_submission_digest: prepared.submission_digest,
    },
    201,
  );
}
async function schedule(call, caseId) {
  const context = await call("GET", `/cases/${caseId}/hearings/context`);
  return hearingCommand(call, caseId, {
    operation_id: randomUUID(),
    hearing_id: randomUUID(),
    change: {
      action: "schedule",
      expected_revision: 0,
      expected_case_revision: context.case_revision,
      expected_stage_revision: context.stage_revision,
      values: {
        kind: "initial",
        scheduled_at: "2026-09-01T10:02:03-06:00",
        modality: "in_person",
        venue: "Sede de cita declarada",
        note: null,
        participants: [],
        conviction_basis: null,
      },
    },
  });
}
async function changeHearing(call, caseId, hearing, action) {
  const context =
    action === "replace"
      ? await call("GET", `/cases/${caseId}/hearings/context`)
      : null;
  return hearingCommand(call, caseId, {
    operation_id: randomUUID(),
    hearing_id: hearing.id,
    change: {
      action,
      expected_revision: hearing.revision,
      ...(context
        ? {
            expected_case_revision: context.case_revision,
            expected_stage_revision: context.stage_revision,
            values: { ...hearing.values, venue: "Sede corregida despues" },
          }
        : {}),
      reason: "Cambio organizativo independiente",
    },
  });
}
async function seedResult(call, caseId, hearing) {
  const path = `/cases/${caseId}/hearings/${hearing.id}/results`;
  const command = {
    operation_id: randomUUID(),
    hearing_id: hearing.id,
    result_id: randomUUID(),
    change: {
      action: "record",
      expected_revision: 0,
      anchor_revision: hearing.revision,
      continuation: null,
      values: {
        occurrence: "occurred",
        extent: "partial",
        event_time: { precision: "date", date: "2026-09-01", offset: "-06:00" },
        summary: "Sesion declarada de fixture",
        attendees: [],
        agreements: [],
        provenance: { kind: "operator_note", reference: null, support: null },
      },
    },
  };
  const prepared = await call("POST", `${path}/prepare`, command);
  return call(
    "POST",
    path,
    {
      command: prepared.command,
      expected_submission_digest: prepared.submission_digest,
    },
    201,
  );
}
