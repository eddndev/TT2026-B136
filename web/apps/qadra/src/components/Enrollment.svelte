<script>
  export let enrollment;
  export let ondone;
  let saved = false;
</script>

<section class="enrollment stack" aria-labelledby="enrollment-title">
  <span class="eyebrow">Protege tu acceso</span>
  <h2 id="enrollment-title">Configura el segundo factor</h2>
  <p>
    Agrega esta clave a tu app de autenticaci&oacute;n para <strong>{enrollment.user.email}</strong
    >. La clave y los c&oacute;digos se muestran una sola vez.
  </p>
  <label
    >Clave de configuraci&oacute;n<input readonly value={enrollment.totp_secret_base32} /></label
  >
  <code>{enrollment.totp_secret_base32}</code>
  <details>
    <summary>Ver URI de configuraci&oacute;n manual</summary><code>{enrollment.otpauth_uri}</code>
  </details>
  <h3>C&oacute;digos de recuperaci&oacute;n</h3>
  <p>Guarda estos c&oacute;digos en un lugar seguro. Cada uno funciona una sola vez.</p>
  <div class="recovery-codes">
    {#each enrollment.recovery_codes as code}<code>{code}</code>{/each}
  </div>
  <label class="checkbox"
    ><input type="checkbox" bind:checked={saved} />Ya guard&eacute; la clave y los c&oacute;digos</label
  >
  <button class="primary" disabled={!saved} onclick={ondone}>Finalizar</button>
</section>
