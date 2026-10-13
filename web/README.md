# Qadra web workspace

Install dependencies once from this directory with `npm ci`. The root lockfile
keeps the applications and shared UI on the same dependency versions.

- [apps/qadra](apps/qadra/README.md): existing office portal and its tests.
- [apps/ca](apps/ca/README.md): location of the separate CA operator portal.
- [packages/ui](packages/ui/README.md): shared visual components and brand assets.

The existing root commands `npm run dev`, `npm test`, `npm run test:e2e`,
`npm run build` and `npm run preview` delegate to the Qadra application.
For example, `npm run dev -- --port 4321` forwards the port to that application.
Formatting commands cover Qadra and the shared UI.

Qadra builds to `apps/qadra/dist`. Release packaging still installs those files
under `web/`; the source layout does not relocate production data or services.
Real backend browser acceptance starts at repository root with
`bash scripts/web-demo.sh`. Browser results are under `apps/qadra/test-results*`.

The former sample is archived at
[archive/frontend-prototype](../archive/frontend-prototype/README.md), outside
this workspace. The [documentation index](../docs/README.md) separates current
contracts, operations and historical evidence.
