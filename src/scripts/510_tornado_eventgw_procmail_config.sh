#!/bin/bash
GW_USER=eventgw
PROCMAILRC="/home/$GW_USER/.procmailrc"

#Add eventgw user if eventhandler is not installed/installed later
add_user_group 'eventgw' '/sbin/nologin' '/home/eventgw'

#Add procmailrc for now forwarding to both eventhandler and tornado
if [[ ! -e "$PROCMAILRC" ]] || ! grep "tornado/email.sock" "$PROCMAILRC" >> /dev/null ; then
    echo "[i] Rewriting '$PROCMAILRC' including tornado"
    echo -n 'SHELL=/bin/sh
:0
*
{
    :0 c
    | /usr/bin/socat - /var/run/tornado/email.sock 2>&1
    :0
    | /usr/bin/socat - /var/run/eventhandler/rw/email.socket 2>&1
}' > "$PROCMAILRC"

    chmod 0744 "$PROCMAILRC"
    echo "[+] Done."
fi
