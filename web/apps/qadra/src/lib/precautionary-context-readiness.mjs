// A case without a complete penal profile or a recorded stage has no
// precautionary context yet; its measures and hearings stay readable.
const notReady = ['case_stage_required', 'case_profile_required'];

export async function precautionaryContextOrMissing(read) {
  try {
    return await read();
  } catch (failure) {
    if (notReady.includes(failure?.code)) return null;
    throw failure;
  }
}
