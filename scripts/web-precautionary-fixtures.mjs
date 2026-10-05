// Seed isolated precautionary browser sources through the authenticated public API.
import { randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";

async function account(call, role) {
  const password = `precautionary browser ${role} password`;
  const enrollment = await call(
    "POST",
    "/users",
    {
      email: `precautionary.${role}@example.com`,
      password,
      role,
    },
    201,
  );
  return {
    id: enrollment.user.id,
    email: enrollment.user.email,
    role: enrollment.user.role,
    password,
    recoveryCodes: enrollment.recovery_codes,
  };
}

async function identity(call, path, document, name) {
  const locator = {
    document_id: document.id,
    version: document.version,
    digest: document.digest,
    locator: "Pagina 1 del soporte sintetico de identidad",
  };
  const review = await call("POST", `${path}/participants/proposals/review`, {
    subject: {
      operation: "create",
      values: {
        kind: "natural_person",
        name: { state: "known", value: name },
        curp: {
          state: "unknown",
          reason: "Dato no declarado en el escenario sintetico",
        },
        identity_support: locator,
      },
    },
    participant: { operation: "create" },
    role: {
      organization: null,
      legal_status: null,
      profile: {
        kind: "defendant",
        custody: {
          state: "unknown",
          reason: "Dato no declarado en el escenario sintetico",
        },
      },
      role_support: locator,
    },
    certificate_base64: null,
  });
  const prepared = {
    proposal: review.proposal,
    review: {
      directory_stamp: review.directory_stamp,
      selection_reason: "Nueva identidad declarada en el escenario sintetico",
      different: review.candidates.map((candidate) => ({
        candidate: candidate.reference,
        reason: "Persona distinta declarada en este escenario sintetico",
        support: locator,
      })),
    },
    certificate_base64: null,
  };
  await call("POST", `${path}/participants/proposals/prepare`, prepared);
  const participant = await call(
    "POST",
    `${path}/participants/proposals/commit`,
    {
      prepared,
      signature_base64: null,
    },
    201,
  );
  if (
    participant.case_id !== review.case_id ||
    participant.canonical_format !== "part2" ||
    !participant.subject ||
    participant.subject.case_id !== review.case_id ||
    participant.subject.values.name.value !== name
  )
    throw new Error(
      "precautionary fixture requires an exact typed subject capture",
    );
  return participant;
}

async function initialHearing(call, path, participant, label) {
  const route = `${path}/hearings`;
  const context = await call("GET", `${route}/context`);
  const scheduledAt = new Date(Date.now() - 24 * 3600000)
    .toISOString()
    .replace(/\.\d{3}Z$/, "Z");
  const prepared = await call("POST", `${route}/prepare`, {
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
        venue: `Sede de audiencia inicial ${label}`,
        note: "Convocatoria inicial sintetica para seleccionar su revision exacta",
        participants: [
          { participant_id: participant.id, revision: participant.revision },
        ],
        conviction_basis: null,
      },
    },
  });
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

export async function provisionPrecautionaryHearings(call) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL)
    throw new Error("disposable browser services are required");
  const principal = await call("GET", "/auth/me");
  if (principal.role !== "owner" || !principal.id || !principal.email)
    throw new Error(
      "precautionary fixture requires the authenticated owner principal",
    );
  const fixture = {};
  for (const role of ["owner", "litigator", "paralegal", "client"])
    fixture[role] = await account(call, role);
  const pdf = await readFile(
    new URL(
      "../crates/infrastructure/tests/fixtures/stage-support.pdf",
      import.meta.url,
    ),
  );
  for (const key of ["desktop", "mobile", "policy"]) {
    const reference = `PRECAUTIONARY-${key.toUpperCase()}`;
    const created = await call(
      "POST",
      "/penal-cases",
      {
        title: `Medidas y audiencias cautelares ${key}`,
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
    const metadata = created.administration;
    if (!metadata?.title || metadata.reference !== reference)
      throw new Error(
        "precautionary case response lacks its actual administrative metadata",
      );
    const record = {
      id: created.id,
      title: metadata.title,
      reference: metadata.reference,
    };
    const path = `/cases/${record.id}`;
    for (const role of ["owner", "litigator", "paralegal", "client"])
      await call("PUT", `${path}/members/${fixture[role].id}`, undefined, 204);
    const support = await call("POST", `${path}/documents`, pdf, 201, {
      "X-Document-Name": `precautionary-${key}-support.pdf`,
    });
    const declared = await identity(call, path, support, "Persona declarada");
    const replacement = await identity(
      call,
      path,
      support,
      "Persona sustituta declarada",
    );
    const initial = await initialHearing(call, path, declared, key);
    fixture[key] = {
      case: record,
      support,
      subject: declared.subject,
      replacementSubject: replacement.subject,
      initial,
    };
  }
  return fixture;
}
