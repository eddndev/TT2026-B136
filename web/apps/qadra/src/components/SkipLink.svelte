<script>
  export let target;
</script>

<a
  class="skip-link"
  href="#main-content"
  onclick={(event) => {
    event.preventDefault();
    target()?.focus();
  }}>Saltar al contenido</a
>
