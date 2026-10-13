import { factFailure, factUncertain } from './procedural-fact-errors.mjs';

export function factDraftFailure(failure, writing) {
  const value = { error: factFailure(failure), mode: 'draft' };
  if (
    writing &&
    (factUncertain(failure) || failure.code === 'procedural_fact_operation_conflict')
  ) {
    value.mode = 'uncertain';
    value.error =
      'No se pudo confirmar el resultado. Conservamos el envio para consultar su revision exacta.';
  } else if (failure.code === 'procedural_fact_revision_exhausted') value.mode = 'exhausted';
  else if (
    [
      'procedural_fact_revision_conflict',
      'procedural_fact_already_withdrawn',
      'procedural_fact_not_found',
    ].includes(failure.code)
  )
    value.mode = 'conflict';
  return value;
}
