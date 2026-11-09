<script>
  // Rare filters behind a disclosure. count is how many of them differ from
  // their default, so an unexpected empty result always has a visible cause.
  export let label = 'M\u00e1s filtros';
  export let count = 0;
</script>

<details class="filter-more">
  <summary>{label}{count ? ` (${count})` : ''}</summary>
  <slot />
</details>
