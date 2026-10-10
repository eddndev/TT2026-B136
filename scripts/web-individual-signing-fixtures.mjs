import assert from "node:assert/strict";
import { createHash, randomBytes } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { join } from "node:path";

export async function provisionIndividualSigning(request) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL);
  const certificateDirectory = process.env.TT_LIVE_INDIVIDUAL_CERTIFICATES;
  const privateDirectory = process.env.TT_LIVE_INDIVIDUAL_CREDENTIALS;
  assert.ok(certificateDirectory && privateDirectory);
  const pdf = await readFile(new URL(
    "../crates/infrastructure/tests/fixtures/stage-support.pdf", import.meta.url,
  ));
  const scenarios = {};
  for (const name of ["desktop", "mobile"]) {
    const certificatePath = join(certificateDirectory, `browser-individual-${name}.crt.pem`);
    const privateKeyPath = join(privateDirectory, `individual-${name}.key.pem`);
    assert.match(await readFile(certificatePath, "ascii"), /^-----BEGIN CERTIFICATE-----/);
    assert.equal((await stat(privateKeyPath)).mode & 0o777, 0o600);
    const password = randomBytes(24).toString("hex");
    const enrollment = await request("POST", "/users", {
      email: `web-individual-${name}@example.com`, password, role: "litigator",
    }, 201);
    const record = await request("POST", "/cases", {
      title: `Individual signing ${name}`, reference: `INDIVIDUAL-${name.toUpperCase()}`,
    }, 201);
    const base = `/cases/${record.id}`;
    await request("PUT", `${base}/members/${enrollment.user.id}`, undefined, 204);
    const document = await request("POST", `${base}/documents`, pdf, 201,
      { "X-Document-Name": `individual-${name}.pdf` });
    assert.equal(document.digest, createHash("sha256").update(pdf).digest("hex"));
    assert.equal(document.version, 1);
    assert.equal(document.sealed, false);
    scenarios[name] = {
      litigator: { id: enrollment.user.id, email: enrollment.user.email, password,
        recoveryCodes: enrollment.recovery_codes },
      caseId: record.id, caseTitle: record.title, document, certificatePath, privateKeyPath,
    };
  }
  return scenarios;
}
