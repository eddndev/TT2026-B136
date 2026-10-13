import { factObject as object, factInvalid as invalid } from './procedural-fact-primitives.mjs';
export function deadlinePage(value, rowsKey, limit, cursorKey, idKey, descending = false, cursor) {
  const rows = value?.[rowsKey];
  if (!Array.isArray(rows) || rows.length > limit || typeof value.has_more !== 'boolean')
    invalid('La p\u00e1gina recibida no es v\u00e1lida.');
  const ids = rows.map((row) => row?.[idKey]);
  if (
    ids.some((id) => id == null) ||
    ids.some((id, i) => i > 0 && (descending ? id >= ids[i - 1] : id <= ids[i - 1])) ||
    (cursor !== undefined && ids.some((id) => (descending ? id >= cursor : id <= cursor)))
  )
    invalid('El orden o cursor no corresponde a la consulta.');
  if (
    value.has_more
      ? rows.length !== limit || value[cursorKey] !== ids.at(-1)
      : value[cursorKey] !== null
  )
    invalid('La continuaci\u00f3n de la consulta no coincide.');
  return rows;
}
export function deadlineBudget(value) {
  if (new TextEncoder().encode(JSON.stringify(value)).length > 1048576)
    throw Object.assign(new Error('La declaraci\u00f3n supera el l\u00edmite de 1 MiB.'), {
      status: 413,
    });
}
