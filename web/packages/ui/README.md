# Shared Qadra UI

`@qadra/ui` owns the existing Brand, Icon and Pagination components, base CSS,
and original public brand assets. Applications import these components directly;
they do not maintain copied versions. Astro uses this package's public directory.

This package has no authentication, authorization, account state, or CA commands.
Sharing a visual component grants no access to either application.
