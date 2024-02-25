#!/usr/bin/env bash

echo Process List:
pgrep -l -f awake

echo
echo Power Management:
pmset -g assertions | grep -o -E '[A-Z][a-zA-Z]+ named: "awake"'
