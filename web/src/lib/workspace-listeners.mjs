export function installWorkspaceListeners(onHash, lifecycle) {
  const onVisibility = () => lifecycle.visibilityChanged(document.visibilityState);
  const onActivity = (event) =>
    lifecycle.activity({ isTrusted: event.isTrusted, type: event.type });
  window.addEventListener('hashchange', onHash);
  document.addEventListener('visibilitychange', onVisibility);
  onVisibility();
  const events = ['pointerdown', 'keydown', 'input'];
  events.forEach((name) => document.addEventListener(name, onActivity, true));
  return () => {
    window.removeEventListener('hashchange', onHash);
    document.removeEventListener('visibilitychange', onVisibility);
    events.forEach((name) => document.removeEventListener(name, onActivity, true));
  };
}
