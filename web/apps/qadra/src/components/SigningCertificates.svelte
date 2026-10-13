<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { createSigningDraft } from '../lib/individual-signing-draft.mjs';
  import {
    decodeSigningStatement,
    signingStatementDownload,
  } from '../lib/individual-signing-values.mjs';
  import { signingBinding } from '../lib/individual-signing-responses.mjs';
  import { ownerBase64 } from '../lib/owner-certificate-binary.mjs';
  import { ownerFailure } from '../lib/owner-binding-draft.mjs';
  import { download } from '../lib/documents.mjs';
  import SigningCertificateForm from './SigningCertificateForm.svelte';
  import SigningBindingDetails from './SigningBindingDetails.svelte';
  export let api, user;
  const session = getContext('session-drafts'),
    accountId = user.id;
  const scoped = api.signingCertificates(accountId);
  const blank = () => ({
    bindingId: '',
    prepared: null,
    signature: '',
    mode: 'editing',
    action: 'register',
    original: null,
  });
  let state = blank(),
    current = null,
    receipt = null,
    saved = null;
  let alive = true,
    ready = false,
    busy = false,
    error = '',
    certificate = '',
    confirmed = false;
  const draft = createSigningDraft(session, { accountId }, 'binding', () => ({
    state,
    digest: null,
  }));
  const admitted = () => alive && draft.admitted();
  async function fresh() {
    const principal = await api.me();
    if (!admitted()) return false;
    if (principal.id !== accountId || principal.role !== 'litigator')
      throw Object.assign(new Error('La cuenta cambio.'), { status: 403 });
    const value = await scoped.current();
    if (!admitted()) return false;
    current = value;
    return true;
  }
  async function run(action) {
    if (busy || !admitted()) return;
    busy = true;
    error = '';
    try {
      await action();
    } catch (failure) {
      if ([400, 422].includes(failure.status) && state.mode === 'uncertain') {
        state = { ...state, mode: 'editing', signature: '' };
        confirmed = false;
      }
      if (alive) error = ownerFailure(failure);
      if (failure.status === 403) {
        draft.close();
        ready = false;
        state = blank();
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function load() {
    await run(async () => {
      if (!(await fresh())) return;
      receipt = current;
      saved = draft.pending();
      ready = true;
    });
  }
  function selected(value) {
    if (!admitted() || busy || state.mode === 'uncertain') return;
    certificate = value;
    confirmed = false;
    error = '';
    state = { ...blank(), bindingId: value ? crypto.randomUUID() : '' };
    if (value) draft.register();
  }
  function prepare() {
    if (!certificate || saved || state.mode !== 'editing') return;
    run(async () => {
      if (!(await fresh())) return;
      if (current && !current.withdrawn_at) throw new Error('Ya existe un vinculo activo.');
      const value = await scoped.prepare(state.bindingId, certificate);
      if (admitted()) {
        state = { ...state, prepared: value, signature: '' };
        confirmed = false;
      }
    });
  }
  function complete(value) {
    if (!admitted()) return;
    receipt = current = value;
    state = { ...blank(), mode: 'complete' };
    confirmed = false;
    certificate = '';
    draft.close();
    saved = null;
  }
  function submit() {
    if (!confirmed || !state.prepared || !state.signature || state.mode !== 'editing') return;
    run(async () => {
      if (!(await fresh())) return;
      if (current && !current.withdrawn_at) throw new Error('Consulta el vinculo ya registrado.');
      const data = Object.freeze({
        statement_base64: state.prepared.statement_base64,
        certificate_der_base64: state.prepared.certificate.der_base64,
        signature_base64: state.signature,
      });
      state = { ...state, mode: 'uncertain' };
      complete(await scoped.register(state.bindingId, data));
    });
  }
  function recover() {
    run(async () => {
      const result = await draft.restore(fresh, (value) => {
        const restored = value.state;
        if (!['register', 'withdraw'].includes(restored.action))
          throw new Error('Intento invalido.');
        if (restored.mode === 'uncertain') {
          const original = restored.action === 'register' ? restored.prepared : restored.original;
          const canonical = decodeSigningStatement(original.statement_base64, 'binding');
          if (canonical.accountId !== accountId || canonical.bindingId !== restored.bindingId)
            throw new Error('Intento invalido.');
          if (restored.action === 'register') ownerBase64(restored.signature, 384);
          state = restored;
        } else state = blank();
        confirmed = false;
        certificate = '';
      });
      if (result.status !== 'restored') throw new Error('No se pudo recuperar el intento.');
      saved = null;
    });
  }
  function checkReceipt() {
    run(async () => {
      if (!(await fresh())) return;
      const value = await scoped.get(state.bindingId);
      const original = state.action === 'register' ? state.prepared : state.original;
      await signingBinding(value, accountId, state.bindingId, true);
      if (
        value.statement_base64 !== original.statement_base64 ||
        value.certificate.der_base64 !== original.certificate.der_base64 ||
        (state.action === 'register' && value.signature_base64 !== state.signature)
      )
        throw new Error('El recibo no coincide con este intento.');
      if (state.action === 'withdraw' && !value.withdrawn_at)
        throw new Error('El retiro no esta confirmado.');
      complete(value);
    });
  }
  function startWithdrawal() {
    if (busy || !admitted() || !current || current.withdrawn_at || saved) return;
    state = { ...blank(), action: 'withdraw', bindingId: current.binding_id, original: current };
    confirmed = false;
    draft.register();
  }
  function withdraw() {
    if (!confirmed || state.action !== 'withdraw' || state.mode !== 'editing') return;
    run(async () => {
      if (!(await fresh())) return;
      state = { ...state, mode: 'uncertain' };
      complete(await scoped.withdraw(state.bindingId));
    });
  }
  function discard() {
    if (busy) return;
    draft.close();
    saved = null;
    certificate = '';
    confirmed = false;
    state = blank();
  }
  onMount(load);
  onDestroy(() => {
    draft.dispose(state.mode === 'uncertain');
    alive = false;
    scoped.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">MI CUENTA</span>
    <h1>Mi firma documental</h1>
    <p>Vincula tu certificado publico y firma documentos con tu clave fuera de Qadra.</p>
  </div>
</div>
<p class="notice">
  Perfil interno de demostracion. Solo se reciben certificados publicos y firmas separadas.
</p>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if !ready}<section class="card stack">
    <p role="status">Consultando cuenta y certificado...</p>
    <button class="secondary" disabled={busy} onclick={load}>Volver a consultar</button>
  </section>
{:else}
  {#if saved}<section class="card stack">
      <h2>Intento pendiente del vinculo</h2>
      <p>Recuperarlo comprueba tu cuenta y conserva su identificador. No lo envia.</p>
      <button class="secondary" disabled={busy} onclick={recover}
        >Recuperar intento del vinculo</button
      >
      <button class="secondary" disabled={busy} onclick={discard}
        >Descartar intento pendiente</button
      >
    </section>{/if}
  {#if receipt}<section class="card stack">
      <h2>{receipt.withdrawn_at ? 'Vinculo retirado' : 'Vinculo registrado'}</h2>
      <SigningBindingDetails value={receipt} />
      <p>Retirar el vinculo impide futuras firmas y conserva su evidencia historica.</p>
      <div class="action-row">
        <button
          class="secondary"
          disabled={busy}
          onclick={() =>
            download(
              new Blob([JSON.stringify(receipt)], { type: 'application/json' }),
              'certificate-binding-receipt.json',
            )}>Descargar recibo del vinculo</button
        >
        {#if !receipt.withdrawn_at && state.mode !== 'uncertain' && state.action !== 'withdraw'}
          <button class="secondary" disabled={busy || !!saved} onclick={startWithdrawal}
            >Retirar vinculo</button
          >{/if}
      </div>
    </section>{/if}
  {#if state.mode === 'uncertain'}<section class="card stack">
      <h2>Resultado pendiente de confirmar</h2>
      <p>Conserva este intento y consulta su recibo antes de cualquier otra operacion.</p>
      <code>{state.bindingId}</code><button class="secondary" disabled={busy} onclick={checkReceipt}
        >Consultar recibo del vinculo</button
      >
    </section>
  {:else if !saved && ((state.action === 'withdraw' && state.mode === 'editing') || !current || current.withdrawn_at)}
    <SigningCertificateForm
      {state}
      {busy}
      {certificate}
      bind:confirmed
      oncertificate={selected}
      onprepare={prepare}
      onsubmit={submit}
      onwithdraw={withdraw}
      ondiscard={discard}
      onsignature={(value) => {
        state = { ...state, signature: value };
        confirmed = false;
        error = '';
      }}
      onerror={(failure) => (error = failure.message)}
    />
  {/if}
{/if}
