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
printf "usage: awake [-d] [<duration>]\n" > expected
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

./awake 0m > actual 2>&1
printf "error: invalid duration\n" > expected
test "invalid duration: 0m"

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

killall awake 2>/dev/null

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
cp expected_pm expected
pmset -g assertions | grep -o -E '[A-Z][a-zA-Z]+ named: "awake"' > actual
test "power management: indefinite"
killall awake
