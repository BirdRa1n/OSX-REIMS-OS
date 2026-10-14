# image/ — a ISO (Debian 13 + live-build)

Gera a ISO do OSX-REIMS-OS: Debian 13 (trixie) mínimo, UEFI, que boota no instalador
próprio e, depois de instalado, no console do gerenciador.

## Estrutura

```
image/
  auto/config                          opções do `lb config` (sem d-i, grub-efi, sha256)
  config/
    archives/reims.{list,pref}.*       updates, security e backports (kernel)
    bootloaders/grub-pc/               menu de boot do pendrive + tema
    package-lists/reims.list.chroot    pacotes (sem driver/firmware de GPU, sem Vulkan)
    hooks/live/0100-reims.hook.chroot  permissões, serviços, tema do Plymouth
    includes.chroot/
      usr/lib/reims/brand.sh           nomes e caminhos (a troca de nome começa aqui)
      usr/lib/reims/build-deps.txt     pacotes do build do Reims vGPU
      usr/local/sbin/reims-installer   instalador (whiptail, offline)
      usr/local/sbin/reims-live        tty1 do pendrive: install / rescue / live
      usr/local/sbin/reims-getty       login automático do tty1 (root no live, reims instalado)
      usr/local/sbin/reims-firstboot   primeiro setup
      usr/local/bin/reims-prepare      preparo do ambiente (10 etapas)
      usr/local/bin/reims-session      sessão: cage + foot + reimsctl console
      etc/systemd/system/              reims-firstboot, reims-hwscan, reims-boot-ok, drop-ins
      etc/default/grub.d/reims.cfg     GRUB do sistema instalado (oculto, splash)
      etc/grub.d/01_reims-bootcheck    menu volta após um boot que falhou
      usr/share/plymouth/themes/reims/ tela de carregamento
```

Gerados (gitignored): `config/{binary,bootstrap,chroot,common,source}`, `.build/`,
`cache/`, `includes.chroot/usr/local/bin/reimsctl` e `includes.chroot/usr/lib/reims/version`.

## Como gerar (numa Debian 13)

```bash
sudo apt install -y live-build
scripts/build-iso.sh
```

Sai `image/OSX-REIMS-OS-<versão>-amd64.iso` e o `.sha256`. O CI (`.github/workflows/iso.yml`)
faz o mesmo num container `debian:trixie`, sob demanda e em tags `v*`.

## Gravar

```bash
sudo dd if=OSX-REIMS-OS-0.2.0-amd64.iso of=/dev/sdX bs=4M status=progress oflag=sync
```

No Windows/macOS: balenaEtcher, ou Rufus em **modo DD**. Boot em **UEFI**; se o pendrive
não iniciar, desligue o Secure Boot.
