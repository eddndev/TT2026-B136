<script>
  import Icon from '@qadra/ui/Icon.svelte';
  import { roles } from '../lib/documents.mjs';
  import { viewLabels } from '../lib/workspace.mjs';
  export let user;
  export let view;
  export let onmenu;
</script>

<header class="topbar">
  <div class="topbar-location">
    <button class="icon-button menu-toggle" aria-label="Abrir men&#250;" onclick={() => onmenu()}
      ><Icon name="menu" /></button
    ><span>Mi despacho<span class="breadcrumb">/ {viewLabels[view]}</span></span>
  </div>
  <div class="topbar-identity">
    <span class="badge info"><Icon name="shield" size={13} />Acceso con MFA</span><span
      class="role-label">{roles[user.role] || user.role}</span
    ><span class="avatar" title={user.email}>{user.email.slice(0, 2).toUpperCase()}</span>
  </div>
</header>
