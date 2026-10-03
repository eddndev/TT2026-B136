<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import OwnerCertificateEditor from './OwnerCertificateEditor.svelte';
  import OwnerCertificateReceipt from './OwnerCertificateReceipt.svelte';
  import OwnerCertificateWithdrawal from './OwnerCertificateWithdrawal.svelte';
  import {
    freshOwnerBinding,
    ownerDrafts,
    denyOwnerDrafts,
    ownerFailure,
  } from '../lib/owner-binding-draft.mjs';
  export let api, user;
  const principalId = user.id;
  const session = getContext('session-drafts');
  const scoped = api.ownerCertificates(principalId);
  let alive = true,
    generation = 0,
    busy = false,
    ready = false,
    error = '';
  let current = null,
    receipt = null,
    editor = null,
    withdrawal = null,
    pending = [];
  const admitted = () =>
    alive &&
    session.principal()?.id === principalId &&
    session.principal()?.role === 'owner' &&
    session.canAdmit();
  const active = (request) => admitted() && request === generation;
  function refreshPending() {
    pending = ownerDrafts(session, principalId);
  }
  function denied(failure) {
    if (failure.status !== 403) return;
    denyOwnerDrafts(session);
    current = receipt = editor = withdrawal = null;
    pending = [];
    ready = false;
  }
  async function load() {
    if (!admitted() || busy) return;
    const request = ++generation;
    busy = true;
    ready = false;
    error = '';
    try {
      const context = await freshOwnerBinding(api, scoped, principalId, () => active(request));
      if (!context || !active(request)) return;
      current = context.current;
      receipt = current;
      refreshPending();
      ready = true;
    } catch (failure) {
      if (active(request)) {
        error = ownerFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function openRegistration(saved = null) {
    if (!ready || busy || !admitted() || editor || withdrawal || (!saved && current)) return;
    editor = { bindingId: saved?.resourceId ?? crypto.randomUUID(), saved };
  }
  function openWithdrawal(saved = null) {
    if (!ready || busy || !admitted() || editor || withdrawal || (!saved && !current)) return;
    withdrawal = {
      bindingId: saved?.resourceId ?? current.binding_id,
      saved,
      original: saved ? null : current,
    };
  }
  function confirmed(value) {
    if (!admitted()) return;
    editor = withdrawal = null;
    receipt = value;
    if (!value.withdrawal) current = value;
    else if (current?.binding_id === value.binding_id) current = null;
    refreshPending();
  }
  function closed() {
    editor = withdrawal = null;
    refreshPending();
  }
  function childDenied(failure) {
    error = ownerFailure(failure);
    denied(failure);
  }
  async function readReceipt() {
    if (!ready || busy || !admitted() || !receipt) return;
    const request = ++generation,
      id = receipt.binding_id;
    busy = true;
    error = '';
    try {
      const value = await scoped.get(id);
      if (!active(request)) return;
      receipt = value;
      if (current?.binding_id === id) current = value.withdrawal ? null : value;
    } catch (failure) {
      if (active(request)) {
        error = ownerFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onMount(load);
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">MI CUENTA</span>
    <h1>Mi certificado</h1>
    <p>Vincula tu certificado p&#250;blico a tu propia cuenta con una firma externa.</p>
  </div>
</div>
<p class="notice">
  Perfil interno de demostraci&#243;n. El v&#237;nculo no habilita inicio de sesi&#243;n por
  certificado ni firma individual de documentos.
</p>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if !ready}
  <section class="card">
    <p role="status">Consultando tu cuenta y su v&#237;nculo...</p>
    {#if !busy}<button class="secondary" onclick={load}>Volver a consultar</button>{/if}
  </section>
{:else}
  {#if !editor && !withdrawal}
    {#each pending as saved (saved.key)}<section class="card retained">
        <h2>Intento pendiente</h2>
        <p>Conservado en memoria para esta cuenta. Reabrir no lo env&#237;a.</p>
        <button
          class="secondary"
          onclick={() =>
            saved.action === 'register' ? openRegistration(saved) : openWithdrawal(saved)}
        >
          {saved.action === 'register' ? 'Continuar registro' : 'Continuar retiro'}
        </button>
      </section>{/each}
    {#if !current && !pending.some((row) => row.action === 'register')}
      <section class="card stack">
        <h2>Registrar mi certificado</h2>
        <p>No hay un v&#237;nculo sin retirar para tu cuenta.</p>
        <div class="action-row">
          <button class="primary" onclick={() => openRegistration()}>Registrar certificado</button>
        </div>
      </section>
    {/if}
  {/if}
  {#if receipt}<OwnerCertificateReceipt
      {receipt}
      busy={busy || !!editor || !!withdrawal}
      {admitted}
      canWithdraw={!pending.some((row) => row.action === 'withdraw')}
      onread={readReceipt}
      onwithdraw={() => openWithdrawal()}
    />{/if}
  {#if editor}{#key editor.bindingId}<OwnerCertificateEditor
        {api}
        {scoped}
        {principalId}
        bindingId={editor.bindingId}
        saved={editor.saved}
        onconfirmed={confirmed}
        oncancel={closed}
        ondenied={childDenied}
      />{/key}{/if}
  {#if withdrawal}{#key withdrawal.bindingId}<OwnerCertificateWithdrawal
        {api}
        {scoped}
        {principalId}
        bindingId={withdrawal.bindingId}
        original={withdrawal.original}
        saved={withdrawal.saved}
        onconfirmed={confirmed}
        oncancel={closed}
        ondenied={childDenied}
      />{/key}{/if}
{/if}

<style>
  .retained {
    margin-bottom: 20px;
  }
  .retained p {
    margin: 12px 0;
  }
</style>
