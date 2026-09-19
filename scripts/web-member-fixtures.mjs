import assert from "node:assert/strict";

export async function provisionMembers(call) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL);
  const principal = await call("GET", "/auth/me");
  assert.equal(principal.role, "owner");
  async function account(name, role) {
    const password = `member lifecycle ${name} synthetic password`;
    const result = await call(
      "POST",
      "/users",
      {
        email: `web-members-${name}@example.com`,
        password,
        role,
      },
      201,
    );
    return {
      id: result.user.id,
      email: result.user.email,
      password,
      recoveryCodes: result.recovery_codes,
    };
  }
  const fixture = {
    client: await account("client", "client"),
    selfOwner: await account("self-owner", "owner"),
  };
  for (const key of ["desktop", "mobile"]) {
    const scenario = {
      owner: await account(`${key}-owner`, "owner"),
      subject: await account(`${key}-subject`, "paralegal"),
      available: await account(`${key}-available`, "paralegal"),
    };
    const record = await call(
      "POST",
      "/cases",
      {
        title: `Asignaciones reales ${key}`,
        reference: `MEMBERS-${key.toUpperCase()}`,
      },
      201,
    );
    const base = `/cases/${record.id}`;
    const administration = (await call("GET", `${base}/administration`))
      .administration;
    scenario.case = {
      id: record.id,
      title: administration.title,
      reference: administration.reference,
    };
    for (const member of [scenario.subject, fixture.client, fixture.selfOwner])
      await call("PUT", `${base}/members/${member.id}`, undefined, 204);
    scenario.closed = key === "mobile";
    if (scenario.closed)
      await call("PUT", `${base}/administrative-status`, {
        expected_revision: administration.revision,
        administrative_status: "closed",
      });
    const user = await call("GET", `/users/${scenario.subject.id}`);
    assert.equal(user.active, true);
    assert.equal(user.role, "paralegal");
    assert.match(user.revision, /^(0|[1-9][0-9]*)$/);
    const assigned = await call(
      "GET",
      `${base}/members?limit=20&selection=assigned&email_prefix=${encodeURIComponent(scenario.subject.email)}`,
    );
    assert.equal(assigned.case_id, record.id);
    assert.equal(assigned.items.length, 1);
    assert.equal(assigned.items[0].id, user.id);
    assert.ok(assigned.items[0].assigned_at);
    scenario.assignment = assigned.items[0].assigned_at;
    fixture[key] = scenario;
  }
  return fixture;
}
