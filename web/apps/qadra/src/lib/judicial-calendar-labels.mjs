// Entity labels use the INEGI catalog cited in docs/judicial-calendars-api.md.
export const calendarEntities = [
  ['01', 'Aguascalientes'],
  ['02', 'Baja California'],
  ['03', 'Baja California Sur'],
  ['04', 'Campeche'],
  ['05', 'Coahuila de Zaragoza'],
  ['06', 'Colima'],
  ['07', 'Chiapas'],
  ['08', 'Chihuahua'],
  ['09', 'Ciudad de M\u00e9xico'],
  ['10', 'Durango'],
  ['11', 'Guanajuato'],
  ['12', 'Guerrero'],
  ['13', 'Hidalgo'],
  ['14', 'Jalisco'],
  ['15', 'M\u00e9xico'],
  ['16', 'Michoac\u00e1n de Ocampo'],
  ['17', 'Morelos'],
  ['18', 'Nayarit'],
  ['19', 'Nuevo Le\u00f3n'],
  ['20', 'Oaxaca'],
  ['21', 'Puebla'],
  ['22', 'Quer\u00e9taro'],
  ['23', 'Quintana Roo'],
  ['24', 'San Luis Potos\u00ed'],
  ['25', 'Sinaloa'],
  ['26', 'Sonora'],
  ['27', 'Tabasco'],
  ['28', 'Tamaulipas'],
  ['29', 'Tlaxcala'],
  ['30', 'Veracruz de Ignacio de la Llave'],
  ['31', 'Yucat\u00e1n'],
  ['32', 'Zacatecas'],
];
export const calendarStates = {
  countable: 'Computable',
  excluded: 'Excluido',
  unresolved: 'Sin resolver',
  outside_coverage: 'Fuera de cobertura',
};
export const calendarStatus = { published: 'Publicado', retired: 'Retirado' };
export const calendarWeekdays = [
  'Lunes',
  'Martes',
  'Mi\u00e9rcoles',
  'Jueves',
  'Viernes',
  'S\u00e1bado',
  'Domingo',
];
export const calendarDenied = (failure) => [401, 403].includes(failure.status);
export const calendarUncertain = (failure) => !failure.status || failure.status >= 500;
export const canCalendars = (role, action = 'read') =>
  action === 'manage' ? role === 'owner' : ['owner', 'litigator', 'paralegal'].includes(role);
export function calendarFailure(failure) {
  const messages = {
    judicial_calendar_not_found: 'El calendario o la revisi\u00f3n no est\u00e1n disponibles.',
    judicial_calendar_revision_conflict:
      'La cabeza del calendario cambi\u00f3. Tu borrador se conserva para comparar.',
    judicial_calendar_operation_conflict:
      'La operaci\u00f3n ya fue registrada. Consulta el recibo exacto antes de continuar.',
    judicial_calendar_retired:
      'El calendario fue retirado; conserva su historia y ya no admite cambios.',
    judicial_calendar_revision_exhausted:
      'Este calendario alcanz\u00f3 el l\u00edmite de revisiones.',
    judicial_calendar_scope_change_forbidden:
      'El \u00e1mbito de la primera publicaci\u00f3n es inmutable. Otro \u00e1mbito necesita un calendario nuevo.',
    judicial_calendar_submission_mismatch:
      'La preparaci\u00f3n ya no coincide con el env\u00edo. Revisa sus valores.',
    invalid_judicial_calendar_value:
      'Revisa el \u00e1mbito, las fechas, las fuentes y las reglas declaradas.',
    invalid_judicial_calendar_revision: 'Consulta una revisi\u00f3n v\u00e1lida del calendario.',
    judicial_calendar_body_too_large: 'El calendario supera el l\u00edmite de 1 MiB.',
  };
  return messages[failure.code] || failure.message;
}
