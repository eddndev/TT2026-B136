<script>
  import { onMount, tick } from 'svelte';
  import Auth from './Auth.svelte';
  import Sidebar from './Sidebar.svelte';
  import Overview from './Overview.svelte';
  import Guide from './Guide.svelte';
  import Icon from './Icon.svelte';
  import Documents from './Documents.svelte';
  import Admin from './Admin.svelte';
  import { createApi } from '../lib/api.mjs';
  import { roles } from '../lib/documents.mjs';
  import { normalizeView, viewLabels } from '../lib/workspace.mjs';
  let user = null;
  let documents = [];
  let view = 'overview';
  let documentIntent = null;
  let notice = '';
  let logoutBusy = false;
  let error = '';
  let sidebar;
  let main;
  function reset(message = '') {
    user = null;
    documents = [];
    documentIntent = null;
    view = 'overview';
    notice = message;
    history.replaceState(null, '', '#overview');
  }
  const api = createApi(globalThis.fetch, () =>
    reset('Tu sesi\u00f3n termin\u00f3. Vuelve a iniciar sesi\u00f3n.'),
  );
  async function logout() {
    logoutBusy = true;
    error = '';
    try {
      await api.logout();
      reset();
    } catch (failure) {
      error = failure.message;
    } finally {
      logoutBusy = false;
    }
  }
  async function go(destination) {
    view = normalizeView(destination, user?.role);
    if (location.hash !== `#${view}`) location.hash = view;
    await tick();
    main?.focus({ preventScroll: true });
    window.scrollTo(0, 0);
  }
  function openDocument(intent) {
    documentIntent = intent;
    go('documents');
  }
  onMount(() => {
    const onHash = () => {
      if (user) go(location.hash);
    };
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  });
</script>

{#if !user}<Auth
    {api}
    {notice}
    onlogin={(principal) => {
      user = principal;
      notice = '';
      error = '';
      go(location.hash);
    }}
  />
{:else}
  <a
    class="skip-link"
    href="#main-content"
    onclick={(event) => {
      event.preventDefault();
      main?.focus();
    }}>Saltar al contenido</a
  >
  <div class="app-layout">
    <Sidebar
      bind:this={sidebar}
      {user}
      {view}
      onnavigate={(next) => {
        documentIntent = null;
        go(next);
      }}
      onlogout={logout}
      busy={logoutBusy}
    />
    <div class="app-content">
      <header class="topbar">
        <div class="topbar-location">
          <button
            class="icon-button menu-toggle"
            aria-label="Abrir men&#250;"
            onclick={() => sidebar.open()}><Icon name="menu" /></button
          ><span>Mi despacho<span class="breadcrumb">/ {viewLabels[view]}</span></span>
        </div>
        <div class="topbar-identity">
          <span class="badge info"><Icon name="shield" size={13} />Acceso con MFA</span><span
            class="role-label">{roles[user.role] || user.role}</span
          ><span class="avatar" title={user.email}>{user.email.slice(0, 2).toUpperCase()}</span>
        </div>
      </header>
      <main id="main-content" tabindex="-1" bind:this={main}>
        {#if error}<p class="notice error" role="alert">{error}</p>{/if}
        {#if view === 'overview'}<Overview
            {user}
            {documents}
            onnavigate={go}
            ondocument={openDocument}
          />
        {:else if view === 'documents'}<Documents
            {api}
            {user}
            {documents}
            intent={documentIntent}
            onintent={() => (documentIntent = null)}
            ondocuments={(next) => (documents = next)}
          />
        {:else if view === 'guide'}<Guide onnavigate={go} />
        {:else if user.role === 'owner'}{#key view}<Admin
              {api}
              view={view === 'team' ? 'users' : 'audit'}
            />{/key}{/if}
        <footer class="workspace-footer">
          <span>Qadra / Despacho digital</span><span
            >Entorno local <span class="footer-separator">/</span> TT2026-B136</span
          >
        </footer>
      </main>
    </div>
  </div>
{/if}
