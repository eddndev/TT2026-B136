<script>
  export let enrollment;
  export let ondone;
  let saved = false;
</script>

<section class="enrollment stack" aria-labelledby="enrollment-title">
  <span class="eyebrow">Protege tu acceso</span>
  <h2 id="enrollment-title">Configura el segundo factor</h2>
  <p>
    Agrega esta clave a tu app de autenticacion para <strong>{enrollment.user.email}</strong>. El
    secreto y los codigos se muestran una sola vez.
  </p>
  <label>Clave de configuracion<input readonly value={enrollment.totp_secret_base32} /></label>
  <code>{enrollment.totp_secret_base32}</code>
  <details>
    <summary>Ver URI de configuracion manual</summary><code>{enrollment.otpauth_uri}</code>
  </details>
  <h3>Codigos de recuperacion</h3>
  <p>Guarda estos codigos en un lugar seguro. Cada uno funciona una sola vez.</p>
  <div class="recovery-codes">
    {#each enrollment.recovery_codes as code}<code>{code}</code>{/each}
  </div>
  <label class="checkbox"
    ><input type="checkbox" bind:checked={saved} />Ya guarde el secreto y los codigos</label
  >
  <button class="primary" disabled={!saved} onclick={ondone}>Finalizar</button>
</section>
