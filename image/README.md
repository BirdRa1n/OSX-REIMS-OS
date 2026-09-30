# image/ — build da ISO (Debian 13 + live-build)

Gera `OSX-REIMS-OS` como uma ISO Debian 13 (trixie) sem GUI, que boota direto no
console do `reimsctl` e traz o instalador (disco + senha) do Debian.

## Estrutura (autoral, versionada)

```
image/
  auto/config                         opções do `lb config` (distro, non-free, d-i)
  config/
    package-lists/reims.list.chroot   pacotes (qemu, ovmf, mesa/vulkan, mtools…)
    hooks/live/0100-reims.hook.chroot habilita os serviços systemd
    includes.chroot/
      usr/local/bin/reims-console     loop do console no tty1
      etc/systemd/system/
        reims-provision.service       first-boot (GPU/driver/Vulkan)
        reims-console.service         console no tty1 (conflita com getty@tty1)
    includes.binary/preseed.cfg       preseed do instalador (disco+senha interativos)
```

Gerados pelo `lb`/pelo build (gitignored): `config/{binary,bootstrap,chroot,common,source}`,
`.build/`, `cache/`, e os binários montados em `includes.chroot/usr/local/bin/`
(`reimsctl`, `provision.sh`).

## Como buildar (numa Debian 13)

```bash
sudo apt install -y live-build
scripts/build-iso.sh            # compila reimsctl, monta binários e roda o lb
```

O `build-iso.sh` compila o `reimsctl` (release), copia `reimsctl` + `provision.sh`
para `includes.chroot/usr/local/bin/`, implanta o `reims-vgpu` pré-compilado de
`vgpu/build/` (se existir) e roda o `lb build` (precisa de root/sudo). A ISO sai em
`image/*.iso`.

## Boot / fluxo

1. **Live/instalador**: o menu de boot oferece instalar (d-i) — o usuário escolhe o
   **disco** e define a **senha** (resto preseedado). Ver `includes.binary/preseed.cfg`.
2. **Primeiro boot** (instalado): `reims-provision.service` roda o provisioner
   (GPU/driver/Vulkan) uma vez.
3. **Operação**: `reims-console.service` toma o tty1 e mostra o menu do `reimsctl`.

Ver [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) §4.A–C e [../docs/ROADMAP.md](../docs/ROADMAP.md).
