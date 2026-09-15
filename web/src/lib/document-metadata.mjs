const controls = /[\u0000-\u001f\u007f-\u009f]/;
const whitespace = /^\p{White_Space}+|\p{White_Space}+$/gu;

function value(input, label, limit, optional) {
  if (controls.test(input)) throw new Error(`${label}: no se permiten caracteres de control.`);
  const result = String(input || '').replace(whitespace, '');
  if (!result && !optional) throw new Error(`${label}: no puede estar vac\u00eda.`);
  if ([...result].length > limit) throw new Error(`${label}: usa hasta ${limit} caracteres.`);
  return result || null;
}
export const normalizeTag = (input) => value(input, 'Etiqueta', 40, false);
export function metadataDraft(record = {}) {
  return {
    document_type: record.document_type || '',
    classification: record.classification || '',
    tags: [...(record.tags || [])],
  };
}
export function normalizeMetadata(input) {
  if (input.tags.length > 20) throw new Error('Usa hasta 20 etiquetas.');
  const tags = [...new Set(input.tags.map(normalizeTag))];
  const encoder = new TextEncoder();
  tags.sort((a, b) => {
    const left = encoder.encode(a),
      right = encoder.encode(b);
    for (let i = 0; i < Math.min(left.length, right.length); i++) {
      if (left[i] !== right[i]) return left[i] - right[i];
    }
    return left.length - right.length;
  });
  return {
    document_type: value(input.document_type, 'Tipo de documento', 80, true),
    classification: value(input.classification, 'Clasificaci\u00f3n', 80, true),
    tags,
  };
}
export function metadataFilters(input) {
  const result = {};
  const documentType = value(input.document_type, 'Tipo de documento', 80, true);
  const classification = value(input.classification, 'Clasificaci\u00f3n', 80, true);
  if (documentType) result.document_type = documentType;
  if (classification) result.classification = classification;
  if (input.tag) result.tag = normalizeTag(input.tag);
  return result;
}
export function mutationError(failure, operation = 'guardar') {
  if (!failure.status)
    return `No se pudo confirmar el resultado. Consulta los datos guardados antes de volver a ${operation}.`;
  if (failure.status === 413)
    return 'El archivo o los datos de clasificaci\u00f3n superan el l\u00edmite permitido.';
  return failure.message;
}
