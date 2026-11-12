# StashMango

Cliente git de escritorio. El grafo ocupa el centro; el resto responde a lo que seleccionas.

Licencia [MIT](LICENSE). Copyright (c) 2026 eduartedev.

StashMango ejecuta el `git` del sistema. Si no hay ninguno, la build de Windows puede llevar un MinGit portable. Ese programa es de Git y se distribuye bajo GPL-2; StashMango no lo enlaza. El binario no está en este repositorio.

## Desarrollo

Hace falta Node 22, Rust estable y Git.

```sh
npm install
npm run tauri dev
```

Las pruebas del núcleo:

```sh
cargo test --manifest-path src-tauri/Cargo.toml
```
