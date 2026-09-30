# opencore/ — bases de OpenCore por versão de macOS

Árvore base do OpenCore (config.plist + kexts mínimos) por "rail"
(`macos-13`, `macos-14`, `macos-15`, `macos-26`). A crate `reimsctl-efi` clona a
árvore da versão escolhida, injeta a identidade única (serial/MLB/UUID/ROM/MAC) e
empacota como `OpenCore.qcow2` da VM.

Manter enxuto: a GPU vem do `AppleParavirtGPU.kext` **nativo** do macOS — não é
preciso kext de GPU. Ver [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) §4.E.
