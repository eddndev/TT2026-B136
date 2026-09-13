<script>
  import Icon from './Icon.svelte';
  import Brand from './Brand.svelte';
  import Enrollment from './Enrollment.svelte';
  export let api;
  export let onlogin;
  export let notice = '';
  let email = '';
  let password = '';
  let code = '';
  let challenge = null;
  let deadline = 0;
  let recovery = false;
  let bootstrap = false;
  let enrollment = null;
  let busy = false;
  let error = '';
  let showPassword = false;
  async function submit(event) {
    event.preventDefault();
    busy = true;
    error = '';
    try {
      if (challenge) {
        if (Date.now() >= deadline)
          throw new Error('La solicitud de acceso venci\u00f3. Inicia sesi\u00f3n de nuevo.');
        const session = await api.mfa(
          challenge.challenge_token,
          code.trim(),
          recovery ? 'recovery' : 'totp',
        );
        onlogin(session.user);
      } else if (bootstrap) {
        enrollment = await api.bootstrap(email.trim(), password);
      } else {
        challenge = await api.login(email.trim(), password);
        deadline = Date.now() + challenge.expires_in_seconds * 1000;
      }
    } catch (failure) {
      error = failure.message;
      if (challenge) {
        challenge = null;
        code = '';
      }
    } finally {
      password = '';
      showPassword = false;
      busy = false;
    }
  }
</script>

<main class="auth-layout">
  <div class="auth-card">
    <section class="auth-story">
      <a class="brand" href="/" aria-label="Qadra, inicio"><Brand /></a>
      <div class="story-copy">
        <span class="eyebrow">TU DESPACHO, EN ORDEN</span>
        <h1>Tu trabajo legal,<br /><span>bien resguardado.</span></h1>
        <p>
          Del primer archivo a su evidencia. Un espacio claro para cuidar cada documento de tu
          despacho.
        </p>
        <ul class="auth-benefits">
          <li>
            <span class="benefit-icon"><Icon name="upload" size={20} /></span>
            <div>
              <strong>Documentos bajo resguardo</strong>
              <p>Carga tus archivos y cons&eacute;rvalos cifrados.</p>
            </div>
          </li>
          <li>
            <span class="benefit-icon"><Icon name="shield" size={20} /></span>
            <div>
              <strong>Integridad verificable</strong>
              <p>Sella tus documentos y comprueba su integridad.</p>
            </div>
          </li>
          <li>
            <span class="benefit-icon"><Icon name="download" size={20} /></span>
            <div>
              <strong>Evidencia a la mano</strong>
              <p>Descarga el paquete de evidencia cuando lo necesites.</p>
            </div>
          </li>
        </ul>
      </div>
      <footer><span>Qadra / Evidencia documental</span><span>TT2026-B136</span></footer>
    </section>
    <section class="auth-panel">
      <div class="auth-top">
        <span class="badge neutral">Prototipo local</span><span>Despacho digital</span>
      </div>
      <div class="auth-form">
        {#if enrollment}
          <Enrollment
            {enrollment}
            ondone={() => {
              enrollment = null;
              bootstrap = false;
            }}
          />
        {:else}
          <span class="auth-symbol"><Icon name={challenge ? 'shield' : 'lock'} size={24} /></span>
          <span class="eyebrow"
            >{challenge ? 'PASO 02 / SEGUNDO FACTOR' : 'ACCESO AL DESPACHO'}</span
          >
          <h2>
            {challenge
              ? 'Un paso m\u00e1s.'
              : bootstrap
                ? 'Configura tu despacho.'
                : 'Accede a tu despacho.'}
          </h2>
          <p>
            {challenge
              ? 'Confirma tu identidad para abrir tu espacio de trabajo.'
              : bootstrap
                ? 'Crea la primera cuenta administradora. Disponible solo si no existen usuarios.'
                : 'Entra con tu cuenta para continuar donde lo dejaste.'}
          </p>
          {#if error || notice}<div class="notice error" role="alert">{error || notice}</div>{/if}
          <form class="stack" onsubmit={submit}>
            {#if challenge}
              <label
                >{recovery
                  ? 'C\u00f3digo de recuperaci\u00f3n'
                  : 'C\u00f3digo de 6 d\u00edgitos'}<input
                  autofocus
                  autocomplete="one-time-code"
                  inputmode={recovery ? 'text' : 'numeric'}
                  pattern={recovery ? undefined : '[0-9]{6}'}
                  required
                  bind:value={code}
                /></label
              >
              <small
                >Tienes {Math.round(challenge.expires_in_seconds / 60)} minutos para completar este paso.
                Si se rechaza el c&oacute;digo, inicia sesi&oacute;n de nuevo.</small
              >
              <button class="primary" disabled={busy}
                >{busy ? 'Verificando...' : 'Verificar y entrar'}<Icon
                  name="arrow"
                  size={18}
                /></button
              >
              <button
                class="text-button"
                type="button"
                disabled={busy}
                onclick={() => {
                  recovery = !recovery;
                  code = '';
                }}
                >{recovery
                  ? 'Usar app de autenticaci\u00f3n'
                  : 'Usar c\u00f3digo de recuperaci\u00f3n'}</button
              >
              <button
                class="text-button"
                type="button"
                disabled={busy}
                onclick={() => {
                  challenge = null;
                  code = '';
                }}>Volver al inicio de sesi&oacute;n</button
              >
            {:else}
              <label
                >Correo electr&oacute;nico<input
                  type="email"
                  autocomplete="username"
                  placeholder="tu@despacho.com"
                  required
                  bind:value={email}
                /></label
              >
              <div class="password-field">
                <label for="auth-password">Contrase&ntilde;a</label>
                <div class="password-control">
                  <input
                    id="auth-password"
                    type={showPassword ? 'text' : 'password'}
                    autocomplete={bootstrap ? 'new-password' : 'current-password'}
                    minlength={bootstrap ? 12 : undefined}
                    required
                    bind:value={password}
                  /><button
                    type="button"
                    class="password-toggle"
                    aria-label={showPassword
                      ? 'Ocultar contrase\u00f1a'
                      : 'Mostrar contrase\u00f1a'}
                    aria-pressed={showPassword}
                    aria-controls="auth-password"
                    onclick={() => {
                      showPassword = !showPassword;
                    }}>{showPassword ? 'Ocultar' : 'Mostrar'}</button
                  >
                </div>
              </div>
              {#if bootstrap}<small>Usa al menos 12 caracteres (m&aacute;ximo 1024 bytes).</small
                >{/if}
              <button class="primary" disabled={busy}
                >{busy ? 'Conectando...' : bootstrap ? 'Crear administrador' : 'Continuar'}<Icon
                  name="arrow"
                  size={18}
                /></button
              >
              <div class="auth-switch">
                {bootstrap
                  ? '\u00bfYa tienes una cuenta?'
                  : '\u00bfPrimera vez en el despacho?'}<button
                  class="text-button"
                  type="button"
                  disabled={busy}
                  onclick={() => {
                    bootstrap = !bootstrap;
                    error = '';
                    password = '';
                    showPassword = false;
                  }}>{bootstrap ? 'Iniciar sesi\u00f3n' : 'Configurar acceso inicial'}</button
                >
              </div>
            {/if}
          </form>
          <div class="auth-assurance">
            <Icon name="shield" size={17} /><span
              >Acceso con contrase&ntilde;a y segundo factor.</span
            >
          </div>
        {/if}
      </div>
      <footer>Un espacio de trabajo para tu despacho.</footer>
    </section>
  </div>
</main>
