# shellcheck shell=sh
# O login automático do tty1 vira a sessão do appliance: o instalador no
# pendrive, o console do gerenciador no sistema instalado. Outros terminais
# (Ctrl+Alt+F2, SSH) e REIMS_NO_SESSION=1 ficam com um shell comum.
if [ "$(tty 2>/dev/null)" = /dev/tty1 ] && [ -z "${REIMS_NO_SESSION:-}" ]; then
	if [ -d /run/live/medium ]; then
		[ "$(id -u)" = 0 ] && exec /usr/local/sbin/reims-live
	elif [ -x /usr/local/bin/reims-session ] &&
		[ "$(id -un)" = "$(. /usr/lib/reims/brand.sh && echo "$USER_NAME")" ]; then
		exec /usr/local/bin/reims-session
	fi
fi
