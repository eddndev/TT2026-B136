const text = (key, label, limit = 200) => ({ key, label, type: 'text', limit });
const declared = (field) => ({ ...field, declared: true });
const license = { key: 'license', label: 'C\u00e9dula profesional', type: 'license' };
const choice = (key, label, options) => ({ key, label, type: 'choice', options });
export const profileKinds = [
  {
    key: 'defendant',
    label: 'Imputado',
    fields: [
      declared(
        choice('custody', 'Situaci\u00f3n de libertad declarada', {
          at_liberty: 'En libertad',
          detained: 'Detenido',
        }),
      ),
    ],
  },
  {
    key: 'victim',
    label: 'V\u00edctima u ofendido',
    fields: [
      { key: 'contact', label: 'Contacto seguro declarado', type: 'contact' },
      { key: 'protection', label: 'Protecci\u00f3n declarada', type: 'protection' },
    ],
  },
  {
    key: 'defense_counsel',
    label: 'Defensor',
    fields: [
      license,
      choice('mode', 'Modalidad de defensa', { private: 'Particular', public: 'P\u00fablica' }),
    ],
  },
  {
    key: 'prosecutor',
    label: 'Ministerio P\u00fablico',
    fields: [
      declared(text('office_identifier', 'Identificador de Fiscal\u00eda', 80)),
      declared(text('unit', 'Unidad de adscripci\u00f3n')),
      declared(license),
    ],
  },
  {
    key: 'victim_counsel',
    label: 'Asesor jur\u00eddico',
    fields: [text('institution', 'Instituci\u00f3n o despacho'), declared(license)],
  },
  {
    key: 'control_judge',
    label: 'Juez de control',
    fields: [text('court', '\u00d3rgano jurisdiccional')],
  },
  {
    key: 'trial_court',
    label: 'Tribunal de enjuiciamiento',
    fields: [
      text('judicial_district', 'Distrito judicial'),
      choice('composition', 'Composici\u00f3n del tribunal', {
        single: 'Unipersonal',
        collegiate: 'Colegiado',
      }),
    ],
  },
  {
    key: 'expert',
    label: 'Perito',
    fields: [declared(text('specialty', 'Especialidad')), declared(license)],
  },
  {
    key: 'police',
    label: 'Polic\u00eda',
    fields: [
      declared(text('agency', 'Corporaci\u00f3n')),
      declared(text('unit', 'Unidad de adscripci\u00f3n')),
    ],
  },
  {
    key: 'precautionary_supervisor',
    label: 'Supervisi\u00f3n de medidas cautelares',
    fields: [
      declared(text('authority', 'Autoridad de supervisi\u00f3n')),
      declared(text('unit', 'Unidad de adscripci\u00f3n')),
    ],
  },
  {
    key: 'other',
    label: 'Otro',
    fields: [
      text('label', 'Nombre del rol', 80),
      declared(text('description', 'Descripci\u00f3n del rol')),
    ],
  },
];
export const profileLabel = (kind) => profileKinds.find((item) => item.key === kind)?.label || kind;
export const needsCredential = (kind) => ['defense_counsel', 'control_judge'].includes(kind);
export function fieldDraft(field) {
  const value = field.type === 'license' ? { number: '', issuer: '' } : '';
  if (['contact', 'protection'].includes(field.type)) return { state: 'unknown', reason: '' };
  return field.declared ? { state: 'known', value } : value;
}
