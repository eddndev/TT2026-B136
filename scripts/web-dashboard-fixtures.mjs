import assert from "node:assert/strict";

export async function provisionDashboard(call) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL);
  assert.equal((await call("GET", "/auth/me")).role, "owner");
  const fixture = {};
  for (const role of ["owner", "litigator"]) {
    const password = `dashboard ${role} synthetic password`;
    const enrollment = await call(
      "POST",
      "/users",
      {
        email: `web-dashboard-${role}@example.com`,
        password,
        role,
      },
      201,
    );
    fixture[role] = {
      id: enrollment.user.id,
      email: enrollment.user.email,
      password,
      recoveryCodes: enrollment.recovery_codes,
    };
  }
  for (const name of ["visible", "hidden"]) {
    const record = await call(
      "POST",
      "/cases",
      {
        title: `Indicadores reales ${name}`,
        reference: `DASHBOARD-${name.toUpperCase()}`,
      },
      201,
    );
    const base = `/cases/${record.id}`;
    const administration = (await call("GET", `${base}/administration`))
      .administration;
    assert.equal(administration.administrative_status, "active");
    fixture[`${name}Case`] = {
      id: record.id,
      title: administration.title,
      reference: administration.reference,
    };
    const document = await call(
      "POST",
      `${base}/documents`,
      Buffer.from(`Synthetic dashboard contract ${name}\n`),
      201,
      { "X-Document-Name": `dashboard-${name}-contract.txt` },
    );
    assert.equal(document.sealed, false);
    await call("PUT", `${base}/documents/${document.id}/metadata`, {
      expected_metadata_revision: 0,
      document_type: name === "visible" ? "Contrato" : "CONTRACT",
      classification: "Demostracion",
      tags: ["tablero"],
    });
    if (name === "visible")
      await call(
        "PUT",
        `${base}/members/${fixture.litigator.id}`,
        undefined,
        204,
      );
  }
  return fixture;
}
