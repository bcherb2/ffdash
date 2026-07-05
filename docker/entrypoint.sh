#!/bin/bash
set -e

PWD_ENV="${SSH_PASSWORD:-docker}"
echo "root:${PWD_ENV}" | chpasswd

# Enable root login via password (Debian/Ubuntu default denies it)
sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config
sed -i 's/PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config

# Ensure SSH host keys exist
if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
    ssh-keygen -A
fi

# Start SSHD in background
/usr/sbin/sshd

# Default command
if [ "$#" -eq 0 ]; then
    exec ffdash /videos
fi

case "$1" in
    ffdash|--*)
        exec ffdash "$@"
        ;;
    bash|/bin/bash|sh|/bin/sh)
        exec "$@"
        ;;
    *)
        exec "$@"
        ;;
esac
