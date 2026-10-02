// Independent fixture families share an authenticated provisioner, not cases.
export async function provisionFixtures(tasks, workers = 1) {
  if (!Number.isInteger(workers) || workers < 1 || workers > 2)
    throw new Error("Fixture provisioning requires one or two workers");
  const entries = Object.entries(tasks);
  const result = {};
  for (let index = 0; index < entries.length; index += workers) {
    const batch = entries.slice(index, index + workers);
    const outcomes = await Promise.allSettled(
      batch.map(async ([name, provision]) => {
        const started = performance.now();
        const value = await provision();
        console.info(
          `Fixture ${name}: ${((performance.now() - started) / 1000).toFixed(2)}s`,
        );
        return value;
      }),
    );
    const failure = outcomes.find((outcome) => outcome.status === "rejected");
    if (failure) throw failure.reason;
    for (let offset = 0; offset < batch.length; offset += 1)
      result[batch[offset][0]] = outcomes[offset].value;
  }
  return result;
}
