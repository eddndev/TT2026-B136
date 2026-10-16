import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";
import { provisionLocalSigningKey } from "./web-local-key-fixtures.mjs";

export async function provisionOwnerCertificates(request) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL);
  const certificatePath = process.env.TT_LIVE_OWNER_CERTIFICATE;
  assert.ok(
    certificatePath,
    "An isolated public Owner certificate is required",
  );
  const localSigning = await provisionLocalSigningKey(
    process.env.TT_LIVE_OWNER_CERTIFICATE_PRIVATE_KEY,
  );
  const password = randomBytes(24).toString("hex");
  const enrollment = await request(
    "POST",
    "/users",
    {
      email: "web-owner-certificate@example.com",
      password,
      role: "owner",
    },
    201,
  );
  return {
    owner: {
      id: enrollment.user.id,
      email: enrollment.user.email,
      password,
      recoveryCodes: enrollment.recovery_codes,
    },
    certificatePath,
    localSigning,
  };
}
