export function discardDocumentDrafts(registry, { contextId, editorKey }, failure) {
  if (![403, 404].includes(failure?.status)) return null;
  if (failure.code === 'case_not_found') {
    registry.denyContext(contextId);
    return 'context';
  }
  registry.closeEditor(editorKey);
  return 'editor';
}
