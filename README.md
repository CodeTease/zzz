# zzz 💤

> A fancy sleep command written in Rust.

`zzz` (executed as `zzs`) is an interactive, feature-packed replacement for the traditional `sleep` command. It offers progress bars, custom themes, Pomodoro timers, process watching, flexible duration inputs, command execution post-timer, and keyboard controls.

A **CodeTease** project.

---

## Installation

Install from [crates.io](https://crates.io/crates/zzzsleep):

```bash
cargo install zzzsleep
```

Please read the [Installation Guide](INSTALLATION.md) for detailed installation instructions.

---

## Features

- ⏳ **Flexible Sleep Duration**: Pass standard durations like `5s`, `1.5m`, `2h`, or specify target times with `--until` (e.g. `17:00`, `5:30pm`, `tomorrow-8am`).
- ⏱️ **Stopwatch Mode**: Track elapsed time upwards with `--stopwatch` and log lap times on the fly.
- 🍅 **Pomodoro Mode**: Built-in Pomodoro productivity timer cycles (work/break intervals).
- 🎨 **Visual Themes**: Choose from multiple animated themes (`classic`, `cat`, `moon`, `pixel`, `matrix`). Respects `NO_COLOR` standard.
- ⌨️ **Interactive Terminal Controls**: Pause/resume, adjust remaining time (`+`/`-`), skip, reset, or record lap times (`l`) interactively.
- 👁️ **Process Monitoring**: Watch a process PID (`--watch <PID>`) and exit automatically if the monitored process terminates.
- 🚀 **Command Execution & Hooks**: Automatically run commands upon completion (`--then`), pause (`--on-pause`), tick (`--on-tick`), or cancellation (`--on-interrupt`).
- 📋 **Interactive Action Menu**: Choose post-timer actions (`--then-menu`) interactively (Lock screen, Shutdown, Do nothing, Custom command).
- 🛠️ **Scripting & CI Support**: Non-interactive mode (`--no-interaction`), quiet mode (`--quiet`), and raw output mode (`--raw`) for seamless script and pipeline integration.
- ⚙️ **Environment Variable Config**: Configure options via environment variables (`ZZZ_*`).

---

## Usage

### Basic Sleep
Sleep for a specific duration:
```bash
zzs 10s
zzs 1.5m
zzs 2h
```

### Sleep Until Specific Time
Sleep until a target time today or tomorrow:
```bash
zzs --until 17:00
zzs --until 5:30pm
zzs --until tomorrow-8am
```

### Pomodoro Timer
Start a Pomodoro cycle with default 25m work / 5m break intervals:
```bash
zzs --pomo
```
Customize work and break durations:
```bash
zzs --pomo --pomo-work 50m --pomo-break 10m
```

### Themes
Choose a visual theme for the progress bar:
```bash
zzs 5m --theme cat
zzs 10m --theme moon
zzs 1h --theme matrix
zzs 30s --theme pixel
```
Available themes: `classic`, `cat`, `moon`, `pixel`, `matrix`.

### Count-Up / Stopwatch Mode
Track elapsed time upwards until you press `q` or `Ctrl+C`:
```bash
zzs --stopwatch
```
Press `l` while running to log the lap time to the terminal.

### Hook Callbacks
Execute specific shell commands on pause, tick, or interrupt:
```bash
zzs 25m --then "playerctl play" --on-pause "playerctl pause" --on-interrupt "notify-send 'Cancelled'"
```

### Interactive Command Menu
Present an interactive menu upon timer completion to select a post-timer action:
```bash
zzs 25m --then-menu
```
Presents choices:
- `[1] Lock screen`
- `[2] Shutdown`
- `[3] Do nothing`
- `[4] Run custom command`

### Execute Command After Sleep
Run a shell command when waking up:
```bash
zzs 10m --then "notify-send 'Timer complete!'"
# or using the --exec alias
zzs 1h --exec "make build"
```

### Monitor Process PID
Sleep while monitoring a background process PID. If the process terminates, `zzs` exits immediately:
```bash
zzs 1h --watch 12345
```

### Interactive Controls
While `zzs` is running in an interactive terminal, you can press:
- `Space` or `P`: Pause / Resume timer
- `L`: Record and log lap time (in Stopwatch mode)
- `+` or `=`: Add step duration (default: +30s)
- `-` or `_`: Subtract step duration (default: -30s)
- `S`: Skip timer and complete immediately
- `R`: Reset timer to start
- `Q`: Stop / quit stopwatch timer
- `Ctrl+C` or `C`: Cancel / interrupt timer

Custom step duration:
```bash
zzs 10m --step 1m
```

---

## Command-Line Options

| Option | Environment Variable | Description | Default |
| --- | --- | --- | --- |
| `<DURATION>` | `ZZZ_DURATION` | Sleep duration (e.g. `5s`, `1.5m`, `2h`) | — |
| `--until <TIME>` | `ZZZ_UNTIL` | Sleep until time (e.g. `17:00`, `5:30pm`, `tomorrow-8am`) | — |
| `--pomo` | `ZZZ_POMO` | Enable Pomodoro mode | `false` |
| `--pomo-work <DUR>` | `ZZZ_POMO_WORK` | Work duration for Pomodoro mode | `25m` |
| `--pomo-break <DUR>` | `ZZZ_POMO_BREAK` | Break duration for Pomodoro mode | `5m` |
| `--stopwatch` | `ZZZ_STOPWATCH` | Enable Stopwatch (count-up) mode | `false` |
| `--theme <THEME>` | `ZZZ_THEME` | Progress bar theme (`classic`, `cat`, `moon`, `pixel`, `matrix`) | `classic` |
| `--then / --exec <CMD>` | `ZZZ_THEN` | Execute command upon completion | — |
| `--then-menu` | `ZZZ_THEN_MENU` | Select post-timer action interactively | `false` |
| `--on-interrupt <CMD>` | `ZZZ_ON_INTERRUPT` | Execute command when cancelled / interrupted | — |
| `--on-pause <CMD>` | `ZZZ_ON_PAUSE` | Execute command when timer is paused | — |
| `--on-tick <CMD>` | `ZZZ_ON_TICK` | Execute command on each timer tick | — |
| `--watch <PID>` | `ZZZ_WATCH` | Monitor PID and exit early if process terminates | — |
| `-q, --quiet` | `ZZZ_QUIET` | Hide progress bar / output | `false` |
| `-m, --message <MSG>` | `ZZZ_MESSAGE` | Completion message | `Woke up! 🚀` |
| `--step <DUR>` | `ZZZ_STEP` | Step duration for interactive adjustments | `30s` |
| `--raw` | `ZZZ_RAW` | Concise output format for remaining time | `false` |
| `--no-interaction` | `ZZZ_NO_INTERACTION` | Disable interactive keys and raw terminal mode | `false` |
| `-V, --version` | — | Print version information and exit | — |

---

## License

This project is under the **MIT License**.
