export function documentStatus(document) {
  if (document.report?.verdict === 'not_valid')
    return { label: 'Revisar evidencia', tone: 'danger', key: 'failed' };
  if (document.report?.verdict === 'valid')
    return { label: 'Verificado', tone: 'success', key: 'verified' };
  if (document.sealed === true) return { label: 'Sellado', tone: 'info', key: 'sealed' };
  if (document.sealed === false)
    return { label: 'Pendiente de sello', tone: 'warning', key: 'pending' };
  return { label: 'Estado por confirmar', tone: 'neutral', key: 'unknown' };
}

export function sessionStats(documents) {
  return {
    total: documents.length,
    sealed: documents.filter((item) => item.sealed === true).length,
    pending: documents.filter((item) => item.sealed === false).length,
    verified: documents.filter((item) => item.report?.verdict === 'valid').length,
  };
}

export function filterDocuments(documents, search, status) {
  const query = search.trim().toLowerCase();
  return documents.filter((item) => {
    const matches = `${item.name} ${item.id}`.toLowerCase().includes(query);
    if (status === 'all') return matches;
    if (status === 'sealed') return matches && item.sealed === true;
    return matches && documentStatus(item).key === status;
  });
}

export function normalizeView(hash, role) {
  const view = hash.replace(/^#/, '');
  if (['overview', 'documents', 'guide'].includes(view)) return view;
  if (role === 'owner' && ['team', 'audit'].includes(view)) return view;
  return 'overview';
}

export const viewLabels = {
  overview: 'Inicio',
  documents: 'Documentos',
  team: 'Equipo',
  audit: 'Auditoria',
  guide: 'Guia de uso',
};
