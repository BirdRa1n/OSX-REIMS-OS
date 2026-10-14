# OSX-REIMS-OS — Roadmap

Deriva de [ARCHITECTURE.md](ARCHITECTURE.md). Marque `[x]` conforme concluir.

## v0.2 — paridade de fluxo (código pronto, validação pendente)
- [x] Motor de VM trocado pelo `reims-ctl` do upstream (compilado na máquina, commit fixado).
- [x] Menu de boot: Install, safe graphics, Advanced/Rescue (rescue, live, check, firmware).
- [x] Instalador próprio offline: disco (32/128 GB), `ERASE`, hostname, senha; GPT ESP+root.
- [x] Primeiro setup: rede, hardware, plano de drivers com confirmação, NVIDIA opt-in,
      IOMMU opcional, reboot único, validação READY/SOFTWARE/FAIL; `--force`.
- [x] Sessão kiosk (`cage` + `foot`), menu da sessão, fallback em texto.
- [x] Console: Create (Recovery/Offline/Full), Import, Manage (S/R/F/T), Logs, Maintenance
      (delete, cache, check, prepare, update/rebuild Reims vGPU), autostart.
- [x] Preparo em 10 etapas com volta ao commit anterior se o build falhar.
- [x] `reimsctl hw`, `diagnostics [--save|--splash]`, `version`.
- [x] Plymouth, GRUB oculto com retorno após boot falho, SSH, kernel dos backports.

## Validação (precisa de uma Debian 13 e de hardware real)
- [ ] `lb build` completo (o CI roda num container `debian:trixie`): conferir que o
      live-build usa `config/bootloaders/grub-pc/` também no EFI e que o kernel é achado.
- [ ] Instalação em disco real e em VM (UEFI), inclusive nvme/mmcblk.
- [ ] Primeiro setup em Intel, AMD e NVIDIA; o reboot por firmware.
- [ ] `cage` + `foot` + janela do Reims vGPU (SDL/Wayland) em tela cheia.
- [ ] Preparo completo (build do QEMU do Reims vGPU) e criação de uma VM até o desktop.
- [ ] Plymouth (`plymouth-label`) e o GRUB oculto com `reims_boot_ok`.

## Depois
- [ ] Novo nome do projeto (começa por `brand.sh` e `brand.rs`).
- [ ] Teclado configurável (hoje `us`).
- [ ] Secure Boot (shim/grub assinados).
- [ ] Atualização no lugar (hoje: reinstalar).
