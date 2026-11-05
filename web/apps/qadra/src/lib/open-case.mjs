import { basicCase } from './case-administration.mjs';

// Loads the current administration of one case and returns the record shape
// the case workspace expects; the scoped client is always released.
export async function loadCaseRecord(api, id) {
  const scoped = api.caseAdministration(id);
  try {
    return basicCase(await scoped.get());
  } finally {
    scoped.dispose();
  }
}
