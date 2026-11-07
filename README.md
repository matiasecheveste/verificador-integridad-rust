# Verificador de integridad

Herramienta de línea de comandos en Rust que genera un **manifiesto SHA-256** de una carpeta y luego comprueba si
algún archivo fue **alterado**, **eliminado** o **agregado**. Sirve para controlar entregas de documentos,
respaldos o artefactos de publicación.

**Tecnologías:** Rust 2021, clap (derive), sha2, rayon, walkdir, serde/serde_json, anyhow y GitHub Actions
(Linux, Windows y macOS).

## Qué demuestra

- **Lectura en bloques de 64 KiB**: memoria constante aunque los archivos sean grandes.
- **Cálculo en paralelo** con rayon, uno por archivo.
- **Manifiesto reproducible**: rutas relativas con `/` y ordenadas (`BTreeMap`), idénticas en cualquier sistema.
- **Códigos de salida pensados para automatizar**: `0` íntegro, `1` hay diferencias, `2` error. Con `--json` la
  salida se puede procesar en un pipeline.
- **Manejo de errores** con contexto (`anyhow`) y validación del formato y el algoritmo del manifiesto.
- **Pruebas** unitarias y de integración que ejecutan el binario real; la CI corre `rustfmt`, `clippy -D warnings`
  y las pruebas en tres sistemas operativos, y al publicar una etiqueta `v*` adjunta los binarios a la versión.

## Uso

```bash
cargo build --release

# generar el manifiesto (se excluye a sí mismo si queda dentro de la carpeta)
verificar generar ./entrega -s ./entrega/manifiesto.json --excluir borradores

# comprobar más tarde
verificar comprobar ./entrega -m ./entrega/manifiesto.json --excluir borradores
#   ALTERADO  informe.pdf
#   NUEVO     anexo-3.xlsx
#   12 intactos, 1 alterados, 0 faltantes, 1 nuevos

verificar comprobar ./entrega -m ./entrega/manifiesto.json --json
```

Formato del manifiesto:

```json
{
  "formato": 1,
  "algoritmo": "sha256",
  "archivos": {
    "docs/acta.md": { "sha256": "…", "bytes": 6 }
  }
}
```

## Pruebas

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## Autoría

Desarrollado por **Matías Nicolás Echeveste Cardozo**. Licencia MIT.
