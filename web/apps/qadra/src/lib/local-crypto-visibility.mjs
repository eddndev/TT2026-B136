export function watchLocalVisibility(update) {
  const visible = () => update(document.visibilityState !== 'hidden');
  const hidden = () => update(false);
  document.addEventListener('visibilitychange', visible);
  window.addEventListener('pagehide', hidden);
  window.addEventListener('pageshow', visible);
  visible();
  return () => {
    document.removeEventListener('visibilitychange', visible);
    window.removeEventListener('pagehide', hidden);
    window.removeEventListener('pageshow', visible);
  };
}
