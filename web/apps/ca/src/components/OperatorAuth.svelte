<script>
  import { tick } from 'svelte';
  import Icon from '@qadra/ui/Icon.svelte';

  export let phase;
  export let busy;
  export let error = '';
  export let notice = '';
  export let resetVersion = 0;
  export let onlogin;
  export let onmfa;
  export let oncancel;

  let email = '';
  let password = '';
  let code = '';
  let recovery = false;
  let previousReset = resetVersion;

  $: if (previousReset !== resetVersion) {
    previousReset = resetVersion;
    password = code = '';
    recovery = false;
  }

  function focus(node, enabled) {
    let current = 0;
    function update(allowed) {
      const expected = ++current;
      if (allowed) void tick().then(() => {
        if (expected === current && node.isConnected) node.focus();
      });
    }
    update(enabled);
    return { update, destroy() { current++; } };
  }

  function submit(event) {
    event.preventDefault();
    if (busy) return;
    const submittedPassword = password;
    const submittedCode = code.trim();
    password = code = '';
    if (phase === 'mfa') onmfa(submittedCode, recovery ? 'recovery' : 'totp');
    else onlogin(email.trim(), submittedPassword);
  }

  function changeFactor() {
    recovery = !recovery;
    password = code = '';
  }
</script>

<section class="card access-card" aria-labelledby="access-title">
  <span class="auth-symbol"><Icon name={phase === 'mfa' ? 'shield' : 'lock'} size={23} /></span>
  <span class="eyebrow">{phase === 'mfa' ? 'PASO 02 / SEGUNDO FACTOR' : 'ACCESO DEL OPERADOR'}</span>
  <h2 id="access-title">{phase === 'mfa' ? 'Confirma tu identidad.' : 'Inicia sesi\u00f3n.'}</h2>
  <p class="form-description">
    {phase === 'mfa'
      ? 'Completa el acceso con tu segundo factor.'
      : 'Introduce las credenciales de tu cuenta de operador.'}
  </p>

  {#if phase === 'expired'}
    <div class="notice" role="status">El acceso venci&oacute;. Inicia sesi&oacute;n de nuevo.</div>
  {/if}
  {#if error}<div class="notice error" role="alert">{error}</div>{/if}
  {#if notice}<div class="notice success" role="status">{notice}</div>{/if}

  <form class="stack" onsubmit={submit} aria-busy={busy}>
    {#if phase === 'mfa'}
      <label for="ca-code">
        {recovery ? 'C\u00f3digo de recuperaci\u00f3n' : 'C\u00f3digo de autenticaci\u00f3n'}
        <input
          id="ca-code"
          use:focus={!busy}
          type="text"
          autocomplete="one-time-code"
          inputmode={recovery ? 'text' : 'numeric'}
          pattern={recovery ? undefined : '[0-9]{6}'}
          maxlength={recovery ? 256 : 6}
          aria-describedby="ca-code-help"
          required
          disabled={busy}
          bind:value={code}
        />
      </label>
      <small id="ca-code-help">
        {recovery
          ? 'Introduce uno de tus c\u00f3digos de recuperaci\u00f3n disponibles.'
          : 'Introduce los 6 d\u00edgitos de tu aplicaci\u00f3n de autenticaci\u00f3n.'}
        Si el c&oacute;digo se rechaza, inicia sesi&oacute;n de nuevo.
      </small>
      <button class="primary" type="submit" disabled={busy}>
        {busy ? 'Verificando...' : 'Verificar y entrar'}<Icon name="arrow" size={18} />
      </button>
      <button class="text-button" type="button" disabled={busy} onclick={changeFactor}>
        {recovery ? 'Usar app de autenticaci\u00f3n' : 'Usar c\u00f3digo de recuperaci\u00f3n'}
      </button>
      <button class="text-button" type="button" onclick={oncancel}>
        Volver al inicio de sesi&oacute;n
      </button>
    {:else}
      <label for="ca-email">
        Correo electr&oacute;nico
        <input
          id="ca-email"
          use:focus={!busy}
          type="email"
          autocomplete="username"
          required
          disabled={busy}
          bind:value={email}
        />
      </label>
      <label for="ca-password">
        Contrase&ntilde;a
        <input
          id="ca-password"
          type="password"
          autocomplete="current-password"
          required
          disabled={busy}
          bind:value={password}
        />
      </label>
      <button class="primary" type="submit" disabled={busy}>
        {busy ? 'Conectando...' : 'Continuar'}<Icon name="arrow" size={18} />
      </button>
      {#if busy}
        <button class="text-button" type="button" onclick={oncancel}>Cancelar</button>
      {/if}
    {/if}
  </form>
  <div class="form-assurance"><Icon name="lock" size={15} />Acceso reservado al operador CA.</div>
</section>
