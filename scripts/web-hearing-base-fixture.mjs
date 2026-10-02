// Provision hearing sources and accounts only when their browser scenarios need them.
import { randomUUID } from "node:crypto";

export async function provisionHearings(request, pdf) {
  const hearings = {};
  for (const role of ["owner", "litigator", "paralegal", "client"]) {
    const password = `case hearings browser ${role} password`;
    const enrollment = await request(
      "POST",
      "/users",
      { email: `hearings.${role}@example.com`, password, role },
      201,
    );
    hearings[role] = {
      id: enrollment.user.id,
      email: enrollment.user.email,
      password,
      recoveryCodes: enrollment.recovery_codes,
    };
  }
  for (const [key, title, reference] of [
    ["case", "Audiencias con referencias historicas", "HEARING-WORKFLOW"],
    ["policyCase", "Agenda de audiencia asignada", "HEARING-POLICY"],
    ["trialCase", "Individualizacion con soporte exacto", "HEARING-TRIAL"],
    ["hiddenCase", "Audiencia transversal restringida", "HEARING-HIDDEN"],
  ]) {
    const row = await request(
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
    hearings[key] = { id: row.id, title, reference };
    if (key !== "hiddenCase")
      for (const role of ["litigator", "paralegal", "client"])
        await request(
          "PUT",
          `/cases/${row.id}/members/${hearings[role].id}`,
          undefined,
          204,
        );
  }
  const route = `/cases/${hearings.case.id}`;
  hearings.manual = await request(
    "POST",
    `${route}/participants`,
    { display_name: "Testigo historico", procedural_role: "Testigo" },
    201,
  );
  const identityDocument = await request(
    "POST",
    `${route}/documents`,
    pdf,
    201,
    { "X-Document-Name": "hearing-identity.pdf" },
  );
  const locator = {
    document_id: identityDocument.id,
    version: 1,
    digest: identityDocument.digest,
    locator: "Pagina 1 de soporte sintetico",
  };
  const review = await request(
    "POST",
    `${route}/participants/proposals/review`,
    {
      subject: {
        operation: "create",
        values: {
          kind: "natural_person",
          name: { state: "known", value: "Persona historica de audiencia" },
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
    },
  );
  const prepared = {
    proposal: review.proposal,
    review: {
      directory_stamp: review.directory_stamp,
      selection_reason: "Identidad sintetica revisada",
      different: [],
    },
    certificate_base64: null,
  };
  await request("POST", `${route}/participants/proposals/prepare`, prepared);
  hearings.typed = await request(
    "POST",
    `${route}/participants/proposals/commit`,
    { prepared, signature_base64: null },
    201,
  );
  const trial = `/cases/${hearings.trialCase.id}`;
  const document = await request("POST", `${trial}/documents`, pdf, 201, {
    "X-Document-Name": "hearing-conviction.pdf",
  });
  const support = {
    document_id: document.id,
    version: 1,
    digest: document.digest,
  };
  const at = { precision: "instant", at: "2026-09-01T10:00:00-06:00" };
  await request(
    "POST",
    `${trial}/stage/transitions`,
    {
      expected_revision: 1,
      target: "intermediate",
      accusation_declared_at: at,
      accusation: support,
      note: null,
    },
    201,
  );
  await request(
    "POST",
    `${trial}/stage/transitions`,
    {
      expected_revision: 2,
      target: "trial",
      opening_order_issued_at: at,
      opening_order: support,
      received_at: at,
      receiving_court: "Tribunal sintetico",
      receipt_reference: null,
      receipt_support: null,
      note: null,
    },
    201,
  );
  await request("POST", `${trial}/documents/${document.id}/versions/1/seal`);
  await request(
    "POST",
    `${trial}/documents/${document.id}/versions?expected_version=1`,
    pdf,
    201,
    { "X-Document-Name": "hearing-conviction-later.pdf" },
  );
  hearings.trialSupport = { ...support, name: document.name };
  for (const key of ["policyCase", "hiddenCase"]) {
    const scoped = `/cases/${hearings[key].id}/hearings`,
      context = await request("GET", `${scoped}/context`);
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
          scheduled_at: "2030-10-01T09:02:03-06:00",
          modality: "in_person",
          venue: "Sede sintetica privada",
          note: "Nota que no pertenece a la agenda",
          participants: [],
          conviction_basis: null,
        },
      },
    };
    const preparation = await request("POST", `${scoped}/prepare`, command);
    hearings[`${key}Hearing`] = await request(
      "POST",
      scoped,
      {
        command: preparation.command,
        expected_submission_digest: preparation.submission_digest,
      },
      201,
    );
  }
  return hearings;
}
