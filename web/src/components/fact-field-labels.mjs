export const classLabels = { order: 'Auto u orden', judgment: 'Sentencia', other: 'Otra clase' };
export const characterLabels = {
  personal: 'Personal',
  publication: 'Por publicaci\u00f3n',
  other: 'Otro car\u00e1cter',
};
export const mediumLabels = {
  in_person: 'Presencial',
  electronic: 'Electr\u00f3nico',
  other: 'Otro medio',
};
export const contextLabels = {
  in_hearing: 'En audiencia',
  outside_hearing: 'Fuera de audiencia',
  other: 'Otro contexto',
};
export const outcomeLabels = {
  practiced: 'Practicada segun lo declarado',
  attempted: 'Intentada segun lo declarado',
};
export function factDeclarationLabel(value, catalog = {}) {
  if (value?.kind === 'unknown') return `No consta: ${value.reason}`;
  if (value?.kind !== 'known') return 'Sin declarar';
  if (typeof value.value === 'string') return value.value;
  return value.value?.kind === 'other'
    ? value.value.label
    : catalog[value.value?.kind] || 'Dato declarado';
}
