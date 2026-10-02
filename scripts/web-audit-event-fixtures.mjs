import assert from "node:assert/strict";

// Independent real accounts and real case reads; no synthetic audit history.
export async function provisionAuditEvents(call) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL);
  const fixture = {};
  for (const role of ["owner", "litigator"]) {
    const password = `audit query ${role} synthetic password`;
    const row = await call(
      "POST",
      "/users",
      { email: `web-audit-query-${role}@example.com`, password, role },
      201,
    );
    fixture[role] = {
      id: row.user.id,
      email: row.user.email,
      password,
      recoveryCodes: row.recovery_codes,
    };
  }
  const challenge = await call("POST", "/auth/login", {
    email: fixture.owner.email,
    password: fixture.owner.password,
  });
  const session = await call("POST", "/auth/mfa/recovery", {
    challenge_token: challenge.challenge_token,
    code: fixture.owner.recoveryCodes[7],
  });
  const headers = { Authorization: `Bearer ${session.access_token}` };
  try {
    const record = await call(
      "POST",
      "/cases",
      { title: "Audit query real browser", reference: "AUDIT-LIVE-REAL" },
      201,
      headers,
    );
    fixture.caseId = record.id;
    fixture.resource = `case:${record.id}`;
    for (let index = 0; index < 21; index++)
      await call("GET", `/cases/${record.id}`, undefined, 200, headers);
    const now = Date.now();
    fixture.from = new Date(now - 86400000).toISOString().slice(0, 10);
    fixture.until = new Date(now + 2 * 86400000).toISOString().slice(0, 10);
    const query = new URLSearchParams({
      from: `${fixture.from}T00:00:00Z`,
      until: `${fixture.until}T00:00:00Z`,
      actor: fixture.owner.email,
      action: "case.read",
      resource: fixture.resource,
      limit: "100",
    });
    const page = await call(
      "GET",
      `/audit/events?${query}`,
      undefined,
      200,
      headers,
    );
    assert.equal(page.events.length, 21);
    assert.equal(page.has_more, false);
    assert.equal(page.next_cursor, null);
    assert.equal(new Set(page.events.map((event) => event.sequence)).size, 21);
    for (const event of page.events) {
      assert.equal(event.actor, fixture.owner.email);
      assert.equal(event.action, "case.read");
      assert.equal(event.resource, fixture.resource);
      assert.equal(typeof event.sequence, "string");
    }
    fixture.events = page.events;
  } finally {
    await call("POST", "/auth/logout", undefined, 204, headers);
  }
  return fixture;
}
