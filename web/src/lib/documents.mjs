export const roles = { owner: 'Administrador', litigator: 'Litigante', paralegal: 'Asistente legal', client: 'Cliente' };

export function can(role, action) {
  if (action === 'documents') return ['owner', 'litigator', 'paralegal'].includes(role);
  if (action === 'seal') return ['owner', 'litigator'].includes(role);
  if (['users', 'audit'].includes(action)) return role === 'owner';
  return false;
}

export function validId(value) {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value);
}

export function validateUpload(file, name) {
  if (!file) return 'Selecciona un archivo.';
  if (file.size > 16 * 1024 * 1024) return 'El archivo supera el limite de 16 MiB.';
  if (!name.trim() || !/^[\x20-\x7e]+$/.test(name) || /[\\/]/.test(name) || name === '.' || name === '..') {
    return 'Usa un nombre sin acentos, saltos de linea ni separadores de carpetas.';
  }
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
