<script>
  import { onMount } from 'svelte';
  import Brand from '@qadra/ui/Brand.svelte';
  import Icon from '@qadra/ui/Icon.svelte';
  import OperatorAuth from './OperatorAuth.svelte';
  import OperatorSession from './OperatorSession.svelte';
  import { createCaAccessClient } from '../lib/ca-access-client.mjs';

  let client;
  let ready = false;
  let state = { phase: 'signed-out', operator: null, remainingMs: 0 };
  let busy = false;
  let error = '';
  let notice = '';
  let generation = 0;
  let resetVersion = 0;
  let sessionRequest = false;
  let pendingVisibility = false;
  let lastActivityAt = -Infinity;

  function receiveState(next) {
    const changed = state.phase !== next.phase;
    if (changed) {
      resetVersion++;
      sessionRequest = false;
      pendingVisibility = false;
      error = '';
      notice = '';
    }
    if (next.phase === 'expired' && changed) {
      generation++;
      busy = false;
    }
    if (next.phase === 'active' && changed) lastActivityAt = performance.now();
    state = next;
  }

  async function authenticate(start) {
    if (!ready || busy) return;
    const current = ++generation;
    busy = true;
    error = notice = '';
    try {
      await start();
    } catch (failure) {
      if (current === generation) error = failure.message;
    } finally {
      if (current === generation) busy = false;
    }
  }

  function cancelAuthentication() {
    generation++;
    busy = false;
    error = notice = '';
    resetVersion++;
    client?.cancelAuthentication();
  }

  async function logout() {
    const current = ++generation;
    busy = false;
    sessionRequest = false;
    error = notice = '';
    resetVersion++;
    try {
      const pending = client.logout();
      notice = 'Has cerrado tu sesi\u00f3n en este dispositivo.';
      await pending;
    } catch (failure) {
      if (current === generation) {
        notice = '';
        error = 'Has salido de este dispositivo, pero no se pudo confirmar el cierre en el servidor. '
          + failure.message;
      }
    }
  }

  async function refreshSession(method) {
    if (state.phase !== 'active') return;
    if (sessionRequest) {
      if (method === 'sessionStatus') pendingVisibility = true;
      return;
    }
    const current = generation;
    sessionRequest = true;
    try {
      await client[method]();
      if (current === generation) error = '';
    } catch (failure) {
      if (current === generation && state.phase === 'active') error = failure.message;
    } finally {
      if (current === generation) {
        sessionRequest = false;
        if (pendingVisibility && state.phase === 'active') {
          pendingVisibility = false;
          void refreshSession('sessionStatus');
        }
      }
    }
  }

  function recordInteraction(event) {
    if (!event.isTrusted || document.visibilityState !== 'visible') return;
    if (state.phase !== 'active' || sessionRequest) return;
    if (event.target instanceof Element && event.target.closest('[data-no-activity]')) return;
    const now = performance.now();
    if (now - lastActivityAt < 10_000) return;
    lastActivityAt = now;
    void refreshSession('recordActivity');
  }

  onMount(() => {
    client = createCaAccessClient({ onState: receiveState });
    ready = true;
    const tick = window.setInterval(() => {
      const next = client.snapshot();
      if (next.phase === state.phase) state = next;
    }, 1000);
    const visibility = () => {
      if (document.visibilityState === 'visible') void refreshSession('sessionStatus');
    };
    document.addEventListener('visibilitychange', visibility);
    for (const event of ['keydown', 'pointerdown', 'input'])
      document.addEventListener(event, recordInteraction);
    return () => {
      generation++;
      ready = false;
      window.clearInterval(tick);
      document.removeEventListener('visibilitychange', visibility);
      for (const event of ['keydown', 'pointerdown', 'input'])
        document.removeEventListener(event, recordInteraction);
      client.dispose();
    };
  });
</script>

<a class="skip-link" href="#main-content">Ir al contenido</a>
<div class="ca-shell">
  <header class="topbar">
    <a class="brand" href="/" aria-label="Qadra CA, inicio"><Brand /></a>
    <div class="topbar-context">
      <span class="context-divider" aria-hidden="true"></span>
      <span>Autoridad certificadora</span>
    </div>
    <span class="badge" class:success={state.phase === 'active'}>
      <Icon name="shield" size={15} />
      {state.phase === 'active' ? 'Sesi\u00f3n activa' : 'Acceso del operador'}
    </span>
  </header>

  <main id="main-content" tabindex="-1">
    {#if state.phase === 'active'}
      <OperatorSession
        operator={state.operator}
        remainingMs={state.remainingMs}
        {error}
        onlogout={logout}
        oncommand={(payload) => client.command(payload)}
      />
    {:else}
      <div class="access-layout">
        <section class="access-intro" aria-labelledby="portal-title">
          <span class="eyebrow">QADRA CA</span>
          <h1 id="portal-title">Autoridad<br />certificadora.</h1>
          <p>El espacio de acceso del operador de la autoridad certificadora de Qadra.</p>
          <div class="access-assurance">
            <span class="tile-icon"><Icon name="shield" size={24} /></span>
            <div>
              <strong>Acceso protegido</strong>
              <p>Contrase&ntilde;a y segundo factor para confirmar tu identidad.</p>
            </div>
          </div>
        </section>
        <OperatorAuth
          phase={state.phase}
          busy={busy || !ready}
          {error}
          {notice}
          {resetVersion}
          onlogin={(email, password) => authenticate(() => client.login(email, password))}
          onmfa={(code, mode) => authenticate(() => client.mfa(code, mode))}
          oncancel={cancelAuthentication}
        />
      </div>
    {/if}
  </main>

  <footer class="workspace-footer">
    <span>Qadra CA</span><span>Autoridad certificadora</span>
  </footer>
</div>
