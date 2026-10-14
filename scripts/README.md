# scripts/ — build

- `build-iso.sh` — compila o `reimsctl`, monta o binário e a versão na imagem e roda o
  `live-build` de `image/` → `image/OSX-REIMS-OS-<versão>-amd64.iso` (+ `.sha256`).
  `SKIP_LB=1` só prepara (o CI roda o `lb build` num container `debian:trixie`).

Ver [../image/README.md](../image/README.md).
