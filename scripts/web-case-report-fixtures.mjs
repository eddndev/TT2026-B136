import assert from "node:assert/strict";

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
  const now = Date.now();
  fixture.from = new Date(now - 86400000).toISOString().slice(0, 10);
  fixture.before = new Date(now + 2 * 86400000).toISOString().slice(0, 10);
  return fixture;
}
