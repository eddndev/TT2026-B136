// Admission precedes durable writes in the upload and append application services.
export function uploadOutcomeUncertain(failure) {
  if (failure.status >= 400 && failure.status < 500) return false;
  return !(failure.status === 503 && failure.code === 'document_validator_unavailable');
}
