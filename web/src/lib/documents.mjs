export const roles = {
  owner: 'Administrador',
  litigator: 'Litigante',
  paralegal: 'Asistente legal',
  client: 'Cliente',
};

export function can(role, action) {
  if (action === 'documents') return ['owner', 'litigator', 'paralegal'].includes(role);
  if (action === 'seal') return ['owner', 'litigator'].includes(role);
  if (['users', 'audit'].includes(action)) return role === 'owner';
  return false;
}

export function validId(value) {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value);
}

// Evidence adds .sig and .tsr to names with a 128-byte archive-entry limit.
const MAX_DOCUMENT_NAME = 124;
const reservedNames = new Set([
  'instrucciones.md',
  'certificado.pem',
  'ca.pem',
  'crl.pem',
  'tsa-chain.pem',
]);

export function safeFilename(value) {
  const original = String(value || '')
    .split(/[\\/]/)
    .pop()
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '');
  const dot = original.lastIndexOf('.');
  const extension =
    dot > 0 && /^\.[A-Za-z0-9]{1,19}$/.test(original.slice(dot)) ? original.slice(dot) : '';
  const basename =
    (extension ? original.slice(0, dot) : original)
      .replace(/[^A-Za-z0-9._-]+/g, '-')
      .replace(/^[^A-Za-z0-9]+/, '')
      .replace(/[-.]+$/, '') || 'documento';
  let name = basename + extension;
  // Fixed evidence filenames must remain distinct from the uploaded document.
  if (reservedNames.has(name.toLowerCase())) name = `documento-${name}`;
  if (name.length > MAX_DOCUMENT_NAME) {
    name = name.slice(0, MAX_DOCUMENT_NAME - extension.length) + extension;
  }
  return name;
}

export function validateUpload(file, name) {
  if (!file) return 'Selecciona un archivo.';
  if (file.size > 16 * 1024 * 1024) return 'El archivo supera el limite de 16 MiB.';
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name) || name.length > MAX_DOCUMENT_NAME) {
    return 'Usa hasta 124 caracteres: letras sin acentos, numeros, puntos, guiones o guiones bajos. Empieza con una letra o numero.';
  }
  if (reservedNames.has(name.toLowerCase()))
    return 'Este nombre esta reservado para la evidencia. Usa otro nombre para el documento.';
  return '';
}

export function upsertDocument(documents, document) {
  return [document, ...documents.filter((item) => item.id !== document.id)];
}

export function download(blob, filename) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
