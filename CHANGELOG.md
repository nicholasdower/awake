## 2.1.0

- Print wake time on start.
- Fix off by one issue. Handle 0s.

## 2.0.1

- Kill other processes with library rather than shell.

## 2.0.0

- Improve invalid duration detection.
- Kill other awake processes on startup.
- Replace process with a new invocation using a datetime string.
- Rename `--daemonize` to `--daemon`.
- Add `--kill`.

## 1.2.0

- Fix `--daemonize`. It just didn't work.
- Add support for complex durations like `5h30m`.

## 1.1.2

- Fix version string.

## 1.1.1

- Change package description.

## 1.1.0

- Add `--daemonize`.
- Add support for specifying a duration like `1h`.

## 1.0.0

- Stay awake forever.

