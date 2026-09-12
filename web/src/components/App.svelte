<script>
  import Auth from './Auth.svelte';
  import Icon from './Icon.svelte';
  import Documents from './Documents.svelte';
  import Admin from './Admin.svelte';
  import { createApi } from '../lib/api.mjs';
  import { roles, can } from '../lib/documents.mjs';
  let user = null;
  let documents = [];
  let view = 'documents';
  let notice = '';
  let logoutBusy = false;
  let error = '';
  const api = createApi(globalThis.fetch, () => {
    user = null;
    documents = [];
    view = 'documents';
    notice = 'Tu sesion termino. Vuelve a iniciar sesion.';
  });
  async function logout() {
    logoutBusy = true;
    error = '';
    try {
      await api.logout();
      user = null;
      documents = [];
      view = 'documents';
    } catch (failure) {
      error = failure.message;
    } finally {
      logoutBusy = false;
    }
  }
  const navigation = [
    { id: 'documents', label: 'Documentos', icon: 'file' },
    { id: 'cases', label: 'Expedientes', icon: 'folder' },
    { id: 'audit', label: 'Auditoria', icon: 'shield', permission: 'audit' },
    { id: 'users', label: 'Equipo', icon: 'users', permission: 'users' },
  ];
</script>

{#if !user}<Auth
    {api}
    {notice}
    onlogin={(principal) => {
      user = principal;
      notice = '';
      error = '';
    }}
  />
{:else}
  <div class="app-layout">
    <aside class="sidebar">
      <a class="brand" href="/" aria-label="Folio, inicio"
        ><span class="brand-mark">f.</span> folio<span class="brand-dot">.</span></a
      >
      <div class="workspace-label">
        <span class="workspace-monogram">D</span>
        <div><strong>Mi despacho</strong><small>Espacio de trabajo</small></div>
      </div>
      <span class="eyebrow nav-label">PRINCIPAL</span>
      <nav aria-label="Navegacion principal">
        {#each navigation as item}{#if !item.permission || can(user.role, item.permission)}<button
              class:active={view === item.id}
              aria-current={view === item.id ? 'page' : undefined}
              onclick={() => (view = item.id)}
              ><Icon name={item.icon} />{item.label}{#if item.id === 'cases'}<span class="nav-soon"
                  >Pronto</span
                >{/if}</button
            >{/if}{/each}
      </nav>
      <div class="sidebar-bottom">
        <div class="private-space">
          <Icon name="shield" /><strong>Tu trabajo, protegido</strong>
          <p>Cifrado y evidencia documental desde el servidor.</p>
        </div>
        <div class="profile">
          <span class="avatar">{user.email.slice(0, 2).toUpperCase()}</span>
          <div><strong>{user.email}</strong><small>{roles[user.role] || user.role}</small></div>
        </div>
        <button class="logout" disabled={logoutBusy} onclick={logout}
          ><Icon name="logout" size={17} />{logoutBusy ? 'Cerrando...' : 'Cerrar sesion'}</button
        >
      </div>
    </aside>
    <div class="app-content">
      <header class="topbar">
        <span
          >Mi despacho <span class="breadcrumb"
            >/ {navigation.find((item) => item.id === view)?.label}</span
          ></span
        ><span class="badge neutral"><span class="status-dot"></span>Entorno local</span>
      </header>
      <main id="main-content">
        {#if error}<p class="notice error" role="alert">{error}</p>{/if}
        {#if view === 'documents'}<Documents
            {api}
            {user}
            {documents}
            ondocuments={(next) => (documents = next)}
          />
        {:else if view === 'cases'}<div class="page-heading">
            <div>
              <span class="eyebrow">ORGANIZACION DEL DESPACHO</span>
              <h1>Un lugar para cada expediente</h1>
              <p>La siguiente pieza de tu espacio de trabajo.</p>
            </div>
          </div>
          <section class="card empty-state case-placeholder">
            <Icon name="folder" size={48} /><span class="badge">Proximamente</span>
            <h2>Expedientes en preparacion</h2>
            <p>
              La creacion de casos, participantes y asignaciones aun no esta disponible. Mientras
              tanto, puedes trabajar con documentos y conservar sus identificadores.
            </p>
            <button class="primary" onclick={() => (view = 'documents')}
              >Ir a documentos<Icon name="arrow" size={17} /></button
            >
          </section>
        {:else if can(user.role, view)}{#key view}<Admin {api} {view} />{/key}{/if}
        <footer class="workspace-footer">
          <span>Folio / Despacho digital</span><span
            >TT2026-B136 <span class="footer-separator">/</span> ESCOM - IPN</span
          >
        </footer>
      </main>
    </div>
  </div>
{/if}
