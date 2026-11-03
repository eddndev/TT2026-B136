<script>
  // What a party must do before it can sign: bind a document-signing
  // certificate, renew an expired one, or ask for a request made with its
  // current certificate. Rationale: docs/adr/0086-bilateral-contract-signing.md.
  export let problem, onnavigate;
</script>

<section class="notice stack contract-guide" aria-label="Antes de firmar">
  {#if problem === 'missing'}
    <h3>Antes de firmar, prepara tu firma</h3>
    <ol>
      <li>
        En Mi firma, genera tu clave y su solicitud (CSR) en este navegador y guarda la copia.
      </li>
      <li>Entrega el CSR al responsable de la CA y l&#233;ele la huella de tu clave.</li>
      <li>Recibe tu certificado p&#250;blico y vinc&#250;lalo en Mi firma.</li>
      <li>Firma a mano con el despacho los dos ejemplares de la hoja de huella.</li>
    </ol>
  {:else if problem === 'expired'}
    <h3>Tu certificado venci&#243;</h3>
    <p>Renu&#233;valo y vuelve a vincularlo en Mi firma. Despu&#233;s pide una solicitud nueva.</p>
  {:else}
    <h3>La solicitud usa otro certificado</h3>
    <p>
      Tu firma vinculada no es la que se design&#243; en esta solicitud. Pide al despacho una
      solicitud nueva con tu certificado actual.
    </p>
  {/if}
  <button class="secondary" onclick={() => onnavigate('signing-certificates')}>Ir a Mi firma</button
  >
</section>
