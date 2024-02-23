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
\fBawake\fR \- Stay awake
.SH SYNOPSIS
\fBawake\fR [<duration>]
.SH DESCRIPTION
Stay awake, optionally for the specified duration.
.SH OPTIONS
.TP
\fB\-h, \-\-help\fR
Print help\.
.TP
\fB\-v\, \-\-version\fR
Print the version\.
EOF
