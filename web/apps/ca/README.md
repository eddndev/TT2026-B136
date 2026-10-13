# CA operator portal

This directory is reserved for the approved CA operator application. The current
structure delivery does not enable a CA endpoint or claim operational acceptance.

The implementation will consume `@qadra/ui` and build separately from Qadra.
Its backend will run as the independent `qadra-ca` binary/service on the same
VPS3. Only designated CA operators will have access. Existing PKI operations,
root material and recoverable CRL publication remain the basis of the service.

The finite implementation and acceptance scope is in
[docs/ca/README.md](../../../docs/ca/README.md).
