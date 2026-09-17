// Provision facts through the caller's isolated authenticated HTTP service.
import { randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";

export async function provisionProceduralFacts(call) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL)
    throw new Error("disposable browser services are required");
  const corpus = JSON.parse(
    await readFile(
      new URL(
        "../crates/domain/tests/fixtures/procedural_fact_vectors.json",
        import.meta.url,
      ),
      "utf8",
    ),
  );
  const minimum = (family) =>
    structuredClone(
      corpus.find((row) => row.name === `${family}_minimum`).normalized,
    );
  const result = {};
  for (const role of ["owner", "litigator", "paralegal", "client"]) {
    const password = `procedural facts browser ${role} password`;
    const row = await call(
      "POST",
      "/users",
      {
        email: `procedural-facts.${role}@example.com`,
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
    ["case", "Resoluciones con fuentes historicas", "FACT-WORKFLOW"],
    ["policyCase", "Permisos de hechos declarados", "FACT-POLICY"],
    ["raceCase", "Correcciones concurrentes de hechos", "FACT-RACE"],
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
      display_name: "Persona historica de la notificacion",
      procedural_role: "Testigo",
    },
    201,
  );
  result.archived = await call(
    "PUT",
    `${path}/participants/${result.manual.id}/directory-status`,
    {
      expected_revision: 1,
      directory_status: "archived",
    },
  );
  const pdf = await readFile(
    new URL(
      "../crates/infrastructure/tests/fixtures/stage-support.pdf",
      import.meta.url,
    ),
  );
  const docx = await readFile(
    new URL(
      "../crates/infrastructure/src/document_formats/docx/tests/fixtures/producer.docx",
      import.meta.url,
    ),
  );
  const uploaded = await call("POST", `${path}/documents`, pdf, 201, {
    "X-Document-Name": "fact-source.pdf",
  });
  result.support = {
    document_id: uploaded.id,
    version: uploaded.version,
    digest: uploaded.digest,
  };
  const representation = await call("POST", `${path}/documents`, docx, 201, {
    "X-Document-Name": "fact-representation.docx",
  });
  result.representationSupport = {
    document_id: representation.id,
    version: representation.version,
    digest: representation.digest,
  };
  await call("POST", `${path}/documents/${uploaded.id}/versions/1/seal`);
  await call(
    "POST",
    `${path}/documents/${uploaded.id}/versions?expected_version=1`,
    pdf,
    201,
    { "X-Document-Name": "fact-source-current.pdf" },
  );
  const context = await call("GET", `${path}/hearings/context`);
  const appointment = {
    operation_id: randomUUID(),
    hearing_id: randomUUID(),
    change: {
      action: "schedule",
      expected_revision: 0,
      expected_case_revision: context.case_revision,
      expected_stage_revision: context.stage_revision,
      values: {
        kind: "initial",
        scheduled_at: "2026-09-01T10:00:00-06:00",
        modality: "in_person",
        venue: "Sede declarada de antecedente",
        note: null,
        participants: [],
        conviction_basis: null,
      },
    },
  };
  let prepared = await call("POST", `${path}/hearings/prepare`, appointment);
  result.hearing = await call(
    "POST",
    `${path}/hearings`,
    envelope(prepared),
    201,
  );
  const resultPath = `${path}/hearings/${result.hearing.id}/results`;
  result.agreementId = randomUUID();
  const event = {
    operation_id: randomUUID(),
    hearing_id: result.hearing.id,
    result_id: randomUUID(),
    change: {
      action: "record",
      expected_revision: 0,
      anchor_revision: result.hearing.revision,
      continuation: null,
      values: {
        occurrence: "occurred",
        extent: "partial",
        event_time: { precision: "date", date: "2026-09-01", offset: "-06:00" },
        summary: "Resultado historico declarado como antecedente",
        attendees: [],
        agreements: [
          {
            id: result.agreementId,
            text: "Acuerdo declarado para referencia exacta",
          },
        ],
        provenance: { kind: "operator_note", reference: null, support: null },
      },
    },
  };
  prepared = await call("POST", `${resultPath}/prepare`, event);
  const recorded = await call("POST", resultPath, envelope(prepared), 201);
  prepared = await call("POST", `${resultPath}/prepare`, {
    operation_id: randomUUID(),
    hearing_id: result.hearing.id,
    result_id: recorded.id,
    change: {
      action: "withdraw",
      expected_revision: 1,
      reason: "Retiro de captura conservando antecedente",
    },
  });
  result.hearingResult = await call(
    "POST",
    `${resultPath}/${recorded.id}/withdrawal`,
    envelope(prepared),
    201,
  );
  const values = minimum("resolution");
  values.summary = "Resolucion con antecedente retirado y soporte exacto";
  values.provenance = {
    kind: "hearing_result",
    reference: {
      hearing_id: result.hearing.id,
      result_id: result.hearingResult.id,
      revision: result.hearingResult.revision,
      agreement_id: result.agreementId,
    },
    locator: "Acuerdo declarado",
    support: { ...result.support, locator: "Pagina 1 historica" },
  };
  result.resolution = await fact(call, result.case.id, "resolution", values);
  result.withdrawnResolution = await factChange(
    call,
    result.case.id,
    result.resolution,
    "withdraw",
  );
  const notice = minimum("notification");
  notice.resolution = {
    id: result.resolution.id,
    revision: result.withdrawnResolution.revision,
  };
  notice.summary = "Notificacion historica con dos soportes directos";
  notice.intended_recipient = {
    kind: "known",
    value: {
      kind: "participant",
      id: result.archived.id,
      revision: result.archived.revision,
    },
  };
  notice.actual_receiver = {
    kind: "known",
    value: {
      kind: "unlinked",
      label: "Receptor informado",
      description: "Descripcion declarada sin ficha",
    },
  };
  notice.provenance = {
    kind: "external_reference",
    reference: "Constancia declarada",
    support: { ...result.support, locator: "Pagina 2 constancia" },
  };
  notice.representation = {
    kind: "declared",
    represented: notice.intended_recipient.value,
    representative: notice.actual_receiver.value,
    scope: "Alcance declarado sin validacion juridica",
    provenance: {
      kind: "external_reference",
      reference: "Representacion informada",
      support: { ...result.representationSupport, locator: "Parrafo 1" },
    },
  };
  result.notification = await fact(
    call,
    result.case.id,
    "notification",
    notice,
    result.resolution.id,
  );
  for (const [key, caseKey] of [
    ["policyResolution", "policyCase"],
    ["raceResolution", "raceCase"],
  ]) {
    const simple = minimum("resolution");
    simple.summary = "Resolucion inicial de fixture";
    result[key] = await fact(call, result[caseKey].id, "resolution", simple);
  }
  return result;
}
const envelope = (prepared) => ({
  command: prepared.command,
  expected_submission_digest: prepared.submission_digest,
});
async function fact(call, caseId, family, values, parent) {
  const path = `/cases/${caseId}/resolutions${family === "notification" ? `/${parent}/notifications` : ""}`;
  const command = {
    family,
    operation_id: randomUUID(),
    id: randomUUID(),
    ...(family === "notification" ? { resolution_id: parent } : {}),
    change: { action: "record", expected_revision: 0, values },
  };
  const prepared = await call("POST", `${path}/prepare`, command);
  return call("POST", path, envelope(prepared), 201);
}
async function factChange(call, caseId, row, action) {
  const path = `/cases/${caseId}/resolutions`;
  const command = {
    family: "resolution",
    operation_id: randomUUID(),
    id: row.id,
    change: {
      action,
      expected_revision: row.revision,
      reason: "Retiro administrativo del registro",
    },
  };
  const prepared = await call("POST", `${path}/prepare`, command);
  return call("POST", `${path}/${row.id}/withdrawal`, envelope(prepared), 201);
}
