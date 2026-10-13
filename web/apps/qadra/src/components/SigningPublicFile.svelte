<script>
  import { onDestroy } from 'svelte';
  import { readOwnerPublicCertificate } from '../lib/owner-certificate-files.mjs';
  import { readDetachedSignature } from '../lib/participant-credential-files.mjs';
  import { ownerBlobBase64 } from '../lib/owner-binding-draft.mjs';
  export let kind = 'signature',
    label,
    disabled = false,
    onvalue,
    onerror;
  let generation = 0;
  async function selected(event) {
    const file = event.currentTarget.files?.[0],
      token = ++generation;
    event.currentTarget.value = '';
    if (disabled) return;
    onvalue('');
    if (!file) return;
    try {
      const blob = await (kind === 'certificate'
        ? readOwnerPublicCertificate(file)
        : readDetachedSignature(file));
      const value = await ownerBlobBase64(blob);
      if (token === generation && !disabled) onvalue(value);
    } catch (failure) {
      if (token === generation && !disabled) onerror(failure);
    }
  }
  onDestroy(() => generation++);
</script>

<label class="stack"
  >{label}
  <input
    type="file"
    accept={kind === 'certificate' ? '.pem,.crt' : '.sig,.bin'}
    {disabled}
    onchange={selected}
  />
</label>
