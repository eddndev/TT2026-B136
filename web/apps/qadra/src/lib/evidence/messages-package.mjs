// Spanish messages for the findings about the ZIP file, the package contents
// and the JSON entries of the evidence validator. The codes are the stable
// codes of crates/evidence-verify; tests/evidence-messages.test.mjs keeps the
// table complete. Rationale: docs/adr/0085-in-browser-evidence-validation.md.

/** Shown when the ZIP is not the file Qadra exported. */
export const NOT_QADRA_EXPORT =
  'Este archivo no es el paquete que export\u00f3 Qadra; si lo descomprimiste y volviste a comprimir, descarga de nuevo la evidencia.';

const notQadra = (codes) => Object.fromEntries(codes.map((code) => [code, NOT_QADRA_EXPORT]));

const EXTRA_OR_MISSING = 'El paquete tiene archivos de m\u00e1s o le faltan.';

export const zipMessages = {
  'zip.admitted': 'El archivo ZIP se abri\u00f3 correctamente.',
  'zip.crc_mismatch':
    'Una parte del ZIP no coincide con el control que el propio ZIP guarda: el paquete se modific\u00f3 o se da\u00f1\u00f3 despu\u00e9s de exportarse.',
  ...notQadra([
    'zip.attributes',
    'zip.central_header',
    'zip.comment',
    'zip.data_descriptor',
    'zip.duplicate_name',
    'zip.empty',
    'zip.encrypted',
    'zip.end_record',
    'zip.entry_count',
    'zip.entry_name',
    'zip.entry_too_large',
    'zip.extra_field',
    'zip.flags',
    'zip.header_mismatch',
    'zip.layout',
    'zip.local_header',
    'zip.method',
    'zip.multi_disk',
    'zip.stored_size',
    'zip.timestamp',
    'zip.too_many_entries',
    'zip.total_too_large',
    'zip.version',
    'zip.zip64',
  ]),
};

export const profileMessages = {
  'profile.recognized': 'Se reconoci\u00f3 el tipo de paquete de evidencia.',
  'profile.unrecognized': 'No se reconoci\u00f3 el tipo de paquete de evidencia.',
  'profile.ambiguous': 'El contenido del paquete corresponde a m\u00e1s de un tipo de evidencia.',
  'profile.ambiguous_document': 'No se puede saber cu\u00e1l de los archivos es el documento.',
  'profile.no_document': 'El paquete no incluye el documento.',
  'profile.missing_entry': EXTRA_OR_MISSING,
  'profile.unknown_entry': EXTRA_OR_MISSING,
  'profile.manifest_not_available':
    'Este tipo de paquete incluye un manifiesto que esta versi\u00f3n del validador todav\u00eda no revisa.',
};

export const individualJsonMessages = {
  'individual.json_matches_entries':
    'El resumen de la firma coincide con los dem\u00e1s archivos del paquete.',
  'individual.json_mismatch':
    'El resumen de la firma no coincide con los dem\u00e1s archivos del paquete.',
  'individual_json.byte_range': 'El resumen de la firma tiene un valor fuera de rango.',
  'individual_json.format_version':
    'El resumen de la firma usa una versi\u00f3n que no se reconoce.',
  'individual_json.instant_range': 'El resumen de la firma tiene una fecha fuera de rango.',
  'individual_json.length': 'El resumen de la firma tiene un dato con un tama\u00f1o incorrecto.',
  'individual_json.malformed': 'El resumen de la firma no se puede leer.',
  'individual_json.missing_key': 'Al resumen de la firma le falta un dato.',
  'individual_json.not_object': 'El resumen de la firma no tiene la forma esperada.',
  'individual_json.too_large': 'El resumen de la firma es m\u00e1s grande de lo permitido.',
  'individual_json.type': 'El resumen de la firma tiene un dato de un tipo incorrecto.',
  'individual_json.unknown_key': 'El resumen de la firma tiene un dato que no se reconoce.',
};

const JSON_UNREADABLE = 'Un archivo de datos del paquete no tiene el formato esperado.';

export const jsonMessages = Object.fromEntries(
  [
    'json.control_character',
    'json.duplicate_key',
    'json.integer_out_of_range',
    'json.invalid_escape',
    'json.invalid_utf8',
    'json.leading_zero',
    'json.literal',
    'json.negative_zero',
    'json.not_integer',
    'json.too_deep',
    'json.too_many_values',
    'json.trailing_data',
    'json.unexpected_byte',
    'json.unexpected_end',
    'json.unpaired_surrogate',
  ].map((code) => [code, JSON_UNREADABLE]),
);

export const packageFallbacks = {
  zip: NOT_QADRA_EXPORT,
  profile: 'El contenido del paquete no es el esperado.',
  individual: 'El resumen de la firma no coincide con el paquete.',
  individual_json: 'El resumen de la firma no se puede leer.',
  json: JSON_UNREADABLE,
};
