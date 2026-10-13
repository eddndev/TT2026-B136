<script>
  export let kind;
  const files = { binding: 'certificate-binding', consent: 'document-consent', pdf: 'document' };
  $: name = files[kind];
  $: input = name + (kind === 'pdf' ? '.pdf' : '.bin');
</script>

<details class="technical-detail">
  <summary>Firmar fuera de Qadra</summary>
  <p>
    Usa tu clave RSA de 3072 bits en tu equipo. La clave y su contrasena permanecen fuera de Qadra.
    Firma los bytes del archivo descargado, sin modificarlos.
  </p>
  <pre>openssl dgst -sha256 -sign signer-private.pem -out {name}.sig {input}</pre>
  <p>Selecciona solamente la firma binaria resultante de 384 bytes.</p>
</details>

<style>
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
