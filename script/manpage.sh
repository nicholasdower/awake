#!/usr/bin/env bash

set -e
set -u
set -o pipefail

if [ $# -ne 2 ]; then
  echo "usage: $0 <version> <date>" >&2
  exit 1
fi

version="$1"
date="$2"

rm -rf man
mkdir man
cat << EOF > man/awake.1
.TH AWAKE 1 $date $version ""
.SH NAME
\fBawake\fR \- Keep your Mac awake
.SH SYNOPSIS
\fBawake\fR [-d] [<duration>]
.SH DESCRIPTION
Keep your Mac awake, optionally for the specified duration (e\.g\. 3000s, 300m, 30h, 3d)\.
.SH OPTIONS
.TP
\fB\-d, \-\-daemonize\fR
Daemonize.
.TP
\fB\-h, \-\-help\fR
Print help\.
.TP
\fB\-v\, \-\-version\fR
Print the version\.
EOF
