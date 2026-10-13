import { hearingUncertain } from './hearings.mjs';
import { hearingFailure } from './hearing-errors.mjs';
import { hearingResultFailure } from './hearing-result-errors.mjs';

export function hearingDraftFailure(failure, writing, result = false) {
  const prefix = result ? 'hearing_result' : 'hearing';
  const value = {
    error: (result ? hearingResultFailure : hearingFailure)(failure),
    mode: 'draft',
    issue: '',
  };
  if (writing && (hearingUncertain(failure) || failure.code === `${prefix}_operation_conflict`)) {
    value.mode = 'uncertain';
    value.error =
      'No se pudo confirmar el resultado. Conservamos el envio para consultar su recibo exacto.';
  } else if (failure.code === `${prefix}_revision_exhausted`) value.mode = 'exhausted';
  else if (
    (result
      ? [
          'hearing_result_revision_conflict',
          'hearing_result_already_withdrawn',
          'hearing_result_not_found',
        ]
      : [
          'hearing_revision_conflict',
          'hearing_context_conflict',
          'hearing_already_cancelled',
          'hearing_submission_mismatch',
          'hearing_stage_incompatible',
          'hearing_context_required',
        ]
    ).includes(failure.code)
  )
    value.mode = 'conflict';
  else if (!result) {
    if (failure.code === 'hearing_participant_changed') value.issue = 'participants';
    if (['hearing_support_changed', 'hearing_support_digest_mismatch'].includes(failure.code))
      value.issue = 'support';
  }
  return value;
}
