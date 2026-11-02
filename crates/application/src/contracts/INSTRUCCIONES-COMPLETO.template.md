## 4. Sello del PDF (Td) y del manifiesto (Tf)

```
openssl ts -verify -in contrato.pdf.tsr -data contrato.pdf \
  -CAfile ca.pem -untrusted tsa-chain.pem
openssl ts -verify -in contrato-manifiesto.json.tsr -data contrato-manifiesto.json \
  -CAfile ca.pem -untrusted tsa-chain.pem
```

`contrato-manifiesto.json` lista el SHA-256 y el tamaño de cada entrada del
paquete salvo él mismo, su sello, `contrato-recibo.json` y esta guía.
Compárelos con `sha256sum *`. Tf demuestra que ambas declaraciones y firmas,
T1, Td, los certificados y las CRL existían a esa hora; no demuestra que nadie
leyera el contrato. Los sellos deben cumplir T1 <= Td <= Tf y Tf, con su
exactitud, debe ser anterior al vencimiento firmado.

