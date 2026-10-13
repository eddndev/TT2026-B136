import { refreshActivityReferences } from './resource-activity-draft.mjs';

export async function refreshResourceDeadlineReferences(
  api,
  caseId,
  resourceId,
  selection,
  definition,
  admitted,
) {
  if (!(await refreshActivityReferences(api, caseId, resourceId, selection, admitted))) return null;
  const profiles = api.deadlineProfiles(caseId),
    deadlines = api.deadlines(caseId);
  let profile = null,
    responsible = null,
    source = null,
    calendars = null,
    resolutions = null;
  try {
    if (definition.profile) {
      profile = await profiles.revision(definition.profile.id, definition.profile.revision);
      if (!admitted()) return null;
    }
    if (definition.responsible_id) {
      let afterId;
      do {
        const page = await deadlines.responsibles({ limit: 100, afterId });
        if (!admitted()) return null;
        responsible = page.responsibles.find((row) => row.id === definition.responsible_id) ?? null;
        afterId = page.has_more ? page.next_after_id : undefined;
      } while (!responsible && afterId !== undefined);
    }
    const reference = definition.input.selection.source;
    if (reference.kind === 'known') {
      const value = reference.value;
      if (value.family === 'resolution' && value.revision) {
        source = api.caseResolutions(caseId);
        await source.revision(value.id, value.revision);
      } else if (value.family === 'notification' && value.revision) {
        resolutions = api.caseResolutions(caseId);
        await resolutions.revision(value.resolution.id, value.resolution.revision);
        if (!admitted()) return null;
        source = api.caseNotifications(caseId, value.resolution.id);
        const row = await source.revision(value.id, value.revision);
        if (row.values.resolution.revision !== value.resolution.revision)
          throw new Error('La notificacion no conserva la resolucion seleccionada.');
      } else if (value.family === 'hearing_result' && value.revision) {
        source = api.caseHearingResults(caseId, value.hearing_id);
        const row = await source.revision(value.result_id, value.revision);
        if (
          value.agreement_id !== null &&
          !row.values.agreements.some((entry) => entry.id === value.agreement_id)
        )
          throw new Error('El acuerdo no pertenece al resultado seleccionado.');
      }
      if (!admitted()) return null;
    }
    if (definition.input.calendar) {
      calendars = api.judicialCalendars();
      await calendars.revision(definition.input.calendar.id, definition.input.calendar.revision);
      if (!admitted()) return null;
    }
    return admitted() ? { profile, responsible } : null;
  } finally {
    profiles.dispose();
    deadlines.dispose();
    source?.dispose();
    calendars?.dispose();
    resolutions?.dispose();
  }
}
