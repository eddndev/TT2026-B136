// Remembers, per browser, whether the desktop sidebar is collapsed. Reading
// window.localStorage itself throws when site data is blocked, so the storage
// is obtained inside the guard; the sidebar then simply starts expanded.
const KEY = 'qadra.sidebar.collapsed';
const browserStorage = () => globalThis.localStorage;

export function readSidebarCollapsed(storage = browserStorage) {
  try {
    return storage()?.getItem(KEY) === '1';
  } catch {
    return false;
  }
}

export function storeSidebarCollapsed(collapsed, storage = browserStorage) {
  try {
    storage()?.setItem(KEY, collapsed ? '1' : '0');
  } catch {
    // A preference that cannot be saved only lasts for this page.
  }
}
