import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

// Report actors and memberships are independent of mutable dashboard fixtures.
export async function provisionCaseReports(call) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL);
  assert.equal((await call("GET", "/auth/me")).role, "owner");
  const fixture = { cases: [] };
  for (const role of ["owner", "litigator"]) {
    const password = `case reports ${role} synthetic password`;
    const row = await call(
      "POST",
      "/users",
      {
        email: `web-case-reports-${role}@example.com`,
        password,
        role,
      },
      201,
    );
    fixture[role] = {
      id: row.user.id,
      email: row.user.email,
      password,
      recoveryCodes: row.recovery_codes,
    };
  }
  for (const status of ["active", "closed"]) {
    const row = await call(
      "POST",
      "/cases",
      {
        title: `Informe real ${status}`,
        reference: `REPORT-LIVE-${status.toUpperCase()}`,
      },
      201,
    );
    const base = `/cases/${row.id}`;
    await call(
      "PUT",
      `${base}/members/${fixture.litigator.id}`,
      undefined,
      204,
    );
    let administration = (await call("GET", `${base}/administration`))
      .administration;
    if (status === "closed")
      administration = (
        await call("PUT", `${base}/administrative-status`, {
          expected_revision: administration.revision,
          administrative_status: status,
        })
      ).administration;
    assert.equal(administration.administrative_status, status);
    fixture.cases.push(administration);
  }
  fixture.activity = await provisionActivity(call);
  const now = Date.now();
  fixture.from = new Date(now - 86400000).toISOString().slice(0, 10);
  fixture.before = new Date(now + 2 * 86400000).toISOString().slice(0, 10);
  fixture.activity.from = fixture.from;
  fixture.activity.before = fixture.before;
  return fixture;
}

async function provisionActivity(call) {
  const fixture = {};
  for (const name of ["author", "reader"]) {
    const password = `activity report ${name} synthetic password`;
    const row = await call(
      "POST",
      "/users",
      {
        email: `web-report-activity-${name}@example.com`,
        password,
        role: "litigator",
      },
      201,
    );
    fixture[name] = {
      id: row.user.id,
      email: row.user.email,
      password,
      recoveryCodes: row.recovery_codes,
    };
  }
  const title = "Carga original atribuida tras reasignacion",
    reference = "REPORT-ACTIVITY-LIVE";
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
  fixture.case = { id: row.id, title, reference };
  const base = `/cases/${row.id}`;
  for (const account of [fixture.author, fixture.reader])
    await call("PUT", `${base}/members/${account.id}`, undefined, 204);
  const challenge = await call("POST", "/auth/login", {
    email: fixture.author.email,
    password: fixture.author.password,
  });
  const session = await call("POST", "/auth/mfa/recovery", {
    challenge_token: challenge.challenge_token,
    code: fixture.author.recoveryCodes[7],
  });
  const headers = { Authorization: `Bearer ${session.access_token}` };
  try {
    const principal = await call("GET", "/auth/me", undefined, 200, headers);
    assert.equal(principal.id, fixture.author.id);
    assert.equal(principal.role, "litigator");
    const pdf = await readFile(
      new URL(
        "../crates/infrastructure/tests/fixtures/stage-support.pdf",
        import.meta.url,
      ),
    );
    const document = await call("POST", `${base}/documents`, pdf, 201, {
      ...headers,
      "X-Document-Name": "activity-original.pdf",
    });
    fixture.document = {
      id: document.id,
      version: document.version,
      digest: document.digest,
    };
  } finally {
    await call("POST", "/auth/logout", undefined, 204, headers);
  }
  await call("DELETE", `${base}/members/${fixture.author.id}`, undefined, 204);
  const members = await call(
    "GET",
    `${base}/members?limit=20&selection=assigned`,
  );
  assert.equal(
    members.items.some((member) => member.id === fixture.author.id),
    false,
  );
  assert.equal(
    members.items.some((member) => member.id === fixture.reader.id),
    true,
  );
  return fixture;
}
