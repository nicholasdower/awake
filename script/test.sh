#!/usr/bin/env bash

if [[ -z "$RUNNER_TEMP" ]]; then
  dir="/tmp/awake"
else
  dir="$RUNNER_TEMP/awake"
fi

if [ $# -gt 1 ]; then
  echo "usage: $0 [<bin-path>]" >&2
  exit 1
fi

if [ $# -eq 1 ]; then
  binary="$1/awake"
else
  binary="./target/debug/awake"
fi

if [ ! -f "$binary" ]; then
  echo "error: $binary does not exist" >&2
  exit 1
fi

rm -rf "$dir"
mkdir -p "$dir"
cp "$binary" "$dir"
cd "$dir"

function test() {
  name="$1"
  if diff expected actual > /dev/null; then
    printf "\033[0;32m"
    echo "test passed: $name"
    printf "\033[0m"
  else
    printf "\033[0;31m"
    echo "test failed: $name"
    printf "\033[0m"
    diff expected actual
    exit 1
  fi
}

./awake -h 2>&1 | head -n 1 > actual
printf "usage: awake [-d] [<duration> | <datetime>]\n" > expected
test "help"

./awake 0 > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: 0"

./awake m > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: m"

./awake 1 > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: 1"

./awake 01m > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: 01m"

./awake 1m01s > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: 1m01s"

./awake 1m1m > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: 1m1m"

./awake 1m1d > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: 1m1d"

./awake --foo 2>&1 | head -n 1 > actual
printf "error: unexpected argument found\n" > expected
test "invalid args: --foo"

./awake foo bar 2>&1 | head -n 1 > actual
printf "error: unexpected argument found\n" > expected
test "invalid args: --foo"

./awake 0s > actual 2>&1
printf "" > expected
test "zero duration: 0s"

printf "" > expected
pmset -g assertions | grep -o -E '[A-Z][a-zA-Z]+ named: "awake"' > actual
test "power management: not running"

cat <<-EOF > expected_pm
PreventUserIdleSystemSleep named: "awake"
PreventUserIdleDisplaySleep named: "awake"
PreventSystemSleep named: "awake"
UserIsActive named: "awake"
PreventDiskIdle named: "awake"
EOF

./awake &
sleep 0.1
cp expected_pm expected
pmset -g assertions | grep -o -E '[A-Z][a-zA-Z]+ named: "awake"' > actual
test "power management: indefinite"
killall awake

./awake -d
sleep 0.1
cp expected_pm expected
pmset -g assertions | grep -o -E '[A-Z][a-zA-Z]+ named: "awake"' > actual
test "power management: indefinite daemon"
killall awake

./awake -d 10m
sleep 0.1
pgrep -l -f awake | grep -o -E 'awake --daemon 20[0-9]{2}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}$' >/dev/null
if [ $? -eq 0 ]; then
  printf "\033[0;32mtest passed: daemon: duration\033[0m\n"
else
  printf "\033[0;31mtest failed: daemon: duration\033[0m\n"
  pgrep -l -f awake
  exit 1
fi

killall awake

./awake 10m &
sleep 0.1
pgrep -l -f awake | grep -o -E 'awake 20[0-9]{2}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}$' >/dev/null
if [ $? -eq 0 ]; then
  printf "\033[0;32mtest passed: process: duration\033[0m\n"
else
  printf "\033[0;31mtest failed: process: duration\033[0m\n"
  pgrep -l -f awake
  exit 1
fi

killall awake

./awake 10m &
sleep 0.1
./awake 20m &
sleep 0.1
count=$(pgrep awake | wc -l | tr -d ' ')
if [ $? -eq 0 ]; then
  printf "\033[0;32mtest passed: process: replaces other process\033[0m\n"
else
  printf "\033[0;31mtest failed: process: replaces other process\033[0m\n"
  pgrep -l -f awake
  exit 1
fi

killall awake

./awake 10m &
sleep 0.1
./awake --kill
count=$(pgrep awake | wc -l | tr -d ' ')
if [ $? -eq 0 ]; then
  printf "\033[0;32mtest passed: kill\033[0m\n"
else
  printf "\033[0;31mtest failed: kill\033[0m\n"
  pgrep -l -f awake
  exit 1
fi
