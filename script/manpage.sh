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
\fBawake\fR [-d] [<duration> | <datetime>]
.SH DESCRIPTION
Keep your Mac awake, optionally for the specified duration (e\.g\. 12h30m) or until the specified datetime (e\.g\. 2030-01-01T00:00:00)\.
.SH OPTIONS
.TP
\fB\-d, \-\-daemon\fR
Run as daemon\.
.TP
\fB\-h, \-\-help\fR
Print help\.
.TP
\fB\-v\, \-\-version\fR
Print the version\.
EOF
