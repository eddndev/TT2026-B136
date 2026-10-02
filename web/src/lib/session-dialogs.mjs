// Modal dialogs escape their parent's inert state and must leave the top layer.
export function createSessionDialogs({ root, settled }) {
  let revision = 0;
  let paused = [];
  let focus = null;
  let observer = null;
  function disconnect() {
    observer?.disconnect();
    observer = null;
  }
  function closeOpen(container) {
    const dialogs = [...container.querySelectorAll('dialog[open]')];
    for (const dialog of dialogs) {
      if (!paused.some((entry) => entry.dialog === dialog))
        paused.push({ dialog, modal: dialog.matches(':modal') });
      dialog.close();
    }
  }
  function clear() {
    disconnect();
    paused = [];
    focus = null;
  }
  function transition(phase) {
    const expected = ++revision;
    const container = root();
    if (['hidden', 'checking'].includes(phase)) {
      if (!container) return;
      if (!observer) {
        focus = container.ownerDocument.activeElement;
        observer = new MutationObserver(() => closeOpen(container));
        observer.observe(container, { subtree: true, attributes: true, attributeFilter: ['open'] });
      }
      container.inert = true;
      closeOpen(container);
      if (container.contains(container.ownerDocument.activeElement))
        container.ownerDocument.activeElement.blur();
    } else if (phase === 'active') {
      settled().then(() => {
        if (expected !== revision) return;
        const current = root();
        disconnect();
        if (current) {
          current.inert = false;
          for (const { dialog, modal } of paused) {
            if (current.contains(dialog) && !dialog.open)
              modal ? dialog.showModal() : dialog.show();
          }
          if (focus?.isConnected && current.contains(focus)) focus.focus({ preventScroll: true });
        }
        clear();
      });
    } else clear();
  }
  return {
    transition,
    dispose() {
      revision++;
      clear();
    },
  };
}
