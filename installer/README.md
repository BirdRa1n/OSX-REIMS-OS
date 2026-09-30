# installer/ — instalador Fase 1 (disco + senha)

Instalador mínimo estilo Debian: detecta discos, o usuário escolhe o disco alvo
(aviso de apagamento total) e define a senha; particiona (GPT: ESP + root), grava
o sistema base e o GRUB UEFI, reinicia. **Nada** de configuração de VM/macOS aqui.

Começa como `preseed` do debian-installer (em `../image/preseed/`) e evolui para
uma TUI própria. Ver [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) §4.B e §6.2.
