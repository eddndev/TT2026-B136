// Svelte action for header popovers: calls `close` on Escape or on a pointer
// press outside the node, so every dropdown dismisses the same way.
export function dismissable(node, close) {
  let callback = close;
  const outside = (event) => {
    if (!node.contains(event.target)) callback();
  };
  const escape = (event) => {
    if (event.key === 'Escape') callback(event);
  };
  document.addEventListener('pointerdown', outside, true);
  node.addEventListener('keydown', escape);
  return {
    update(next) {
      callback = next;
    },
    destroy() {
      document.removeEventListener('pointerdown', outside, true);
      node.removeEventListener('keydown', escape);
    },
  };
}
