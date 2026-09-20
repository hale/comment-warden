#!/usr/bin/env bash
set -euo pipefail

# shellcheck disable=SC2086
echo $UNQUOTED_ON_PURPOSE

# shellcheck source=/dev/null
source /etc/profile.d/something.sh

# this line explains nothing that the code doesn't already say
echo "hello"

# TRIPWIRE: this script is invoked by a systemd unit with no TTY — a prompt here hangs the deploy forever
echo "done"
