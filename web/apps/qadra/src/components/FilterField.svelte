<script>
  // A filter label with its control in the slot, so the label text stays the
  // control's accessible name. size: 'wide' for a text search, 'narrow' for
  // short values such as an offset.
  export let size = 'normal';
  let className = '';
  export { className as class };
  $: classes = [size === 'wide' && 'filter-wide', size === 'narrow' && 'filter-narrow', className]
    .filter(Boolean)
    .join(' ');
</script>

<label class={classes || undefined}><slot /></label>
