<script>
  // One row of filters above a list (styles/filter-bar.css). as picks the
  // element (div, form or fieldset); other attributes, such as onsubmit or
  // disabled, pass through to it.
  export let as = 'div';
  let className = '';
  export { className as class };
</script>

<svelte:element this={as} class={`filter-bar ${className}`.trim()} {...$$restProps}
  ><slot /></svelte:element
>
