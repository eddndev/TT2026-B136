// Reuse the disposable HTTP corpus with an independent Owner browser account.
import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import { chmod, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execute = promisify(execFile);

async function capture(base, token, work, repo) {
  const env = { ...process.env };
  for (const prefix of ["TT_FACT_API", "TT_DEADLINE_API"])
    Object.assign(env, {
      [`${prefix}_BASE_URL`]: base,
      [`${prefix}_TOKEN`]: token,
      [`${prefix}_WORK_DIR`]: work,
      [`${prefix}_REPO`]: repo,
    });
  let output,
    failed = false;
  try {
    output = await execute(
      "python3",
      ["-B", join(repo, "scripts/api-legal-corpus-demo.py"), "capture"],
      {
        cwd: repo,
        env,
        timeout: 300000,
        maxBuffer: 4 * 1024 * 1024,
      },
    );
  } catch (error) {
    output = { stdout: error.stdout || "", stderr: error.stderr || "" };
    failed = true;
  }
  const log = join(work, "legal-corpus-capture.log");
  await writeFile(log, `${output.stdout}\n${output.stderr}`, {
    mode: 0o600,
    flag: "wx",
  });
  if (failed)
    throw new Error(`Legal corpus capture failed; inspect private log ${log}`);
  try {
    return JSON.parse(
      await readFile(join(work, "legal-corpus-api-state.json"), "utf8"),
    );
  } catch {
    throw new Error(
      "Legal corpus capture did not produce a readable private state",
    );
  }
}

export async function provisionLegalCorpus(call) {
  const fixturePath = process.env.TT_WEB_FIXTURES,
    base = process.env.API_PROXY_TARGET;
  if (!fixturePath || !base || !process.env.IDENTITY_TEST_DATABASE_URL)
    throw new Error(
      "Disposable browser services are required for the legal corpus",
    );
  const repo = fileURLToPath(new URL("../", import.meta.url));
  const work = await mkdtemp(join(dirname(fixturePath), "legal-corpus-"));
  await chmod(work, 0o700);
  const password = "legal corpus independent owner password";
  const row = await call(
    "POST",
    "/users",
    {
      email: `legal-corpus-${randomUUID()}@example.com`,
      password,
      role: "owner",
    },
    201,
  );
  const owner = {
    id: row.user.id,
    email: row.user.email,
    password,
    recoveryCodes: row.recovery_codes,
  };
  const challenge = await call("POST", "/auth/login", {
    email: owner.email,
    password,
  });
  const session = await call("POST", "/auth/mfa/recovery", {
    challenge_token: challenge.challenge_token,
    code: owner.recoveryCodes[7],
  });
  const headers = { Authorization: `Bearer ${session.access_token}` };
  try {
    const saved = await capture(base, session.access_token, work, repo);
    if (saved.scenario?.actor?.id !== owner.id || !Array.isArray(saved.records))
      throw new Error(
        "Legal corpus capture does not match its independent Owner",
      );
    const record = saved.scenario.case;
    saved.scenario.case = {
      ...record,
      title: record.administration?.title ?? record.title,
      reference: record.administration?.reference ?? record.reference,
    };
    if (!saved.scenario.case.title)
      throw new Error("Legal corpus capture has no navigable case title");
    return { owner, ...saved };
  } finally {
    await call("POST", "/auth/logout", undefined, 204, headers);
  }
}
