# zzz 💤

> A fancy sleep command written in Rust.

`zzz` is an interactive, feature-packed replacement for the traditional `sleep` command. It offers progress bars, custom themes, Pomodoro timers, process watching, flexible duration inputs, command execution post-timer, and keyboard controls.

A **CodeTease** project.

---

## Installation

Please read the [Installation Guide](INSTALLATION.md) for detailed installation instructions.

---

## Features

- ⏳ **Flexible Sleep Duration**: Pass standard durations like `5s`, `1.5m`, `2h`, or specify target times with `--until` (e.g. `17:00`, `5:30pm`, `tomorrow-8am`).
- 🍅 **Pomodoro Mode**: Built-in Pomodoro productivity timer cycles (work/break intervals).
- 🎨 **Visual Themes**: Choose from multiple animated themes (`classic`, `cat`, `moon`, `pixel`, `matrix`). Respects `NO_COLOR` standard.
- ⌨️ **Interactive Terminal Controls**: Pause/resume, adjust remaining time on the fly (`+`/`-`), skip, or reset the timer interactively.
- 👁️ **Process Monitoring**: Watch a process PID (`--watch <PID>`) and exit automatically if the monitored process terminates.
- 🚀 **Command Execution**: Automatically run a command when the timer completes with `--then <COMMAND>` (or `--exec`).
- 🛠️ **Scripting & CI Support**: Non-interactive mode (`--no-interaction`), quiet mode (`--quiet`), and raw output mode (`--raw`) for seamless script and pipeline integration.
- ⚙️ **Environment Variable Config**: Configure options via environment variables (`ZZZ_*`).

---

## Usage

### Basic Sleep
Sleep for a specific duration:
```bash
zzz 10s
zzz 1.5m
zzz 2h
```

### Sleep Until Specific Time
Sleep until a target time today or tomorrow:
```bash
zzz --until 17:00
zzz --until 5:30pm
zzz --until tomorrow-8am
```

### Pomodoro Timer
Start a Pomodoro cycle with default 25m work / 5m break intervals:
```bash
zzz --pomo
```
Customize work and break durations:
```bash
zzz --pomo --pomo-work 50m --pomo-break 10m
```

### Themes
Choose a visual theme for the progress bar:
```bash
zzz 5m --theme cat
zzz 10m --theme moon
zzz 1h --theme matrix
zzz 30s --theme pixel
```
Available themes: `classic`, `cat`, `moon`, `pixel`, `matrix`.

### Execute Command After Sleep
Run a shell command when waking up:
```bash
zzz 10m --then "notify-send 'Timer complete!'"
# or using the --exec alias
zzz 1h --exec "make build"
```

### Monitor Process PID
Sleep while monitoring a background process PID. If the process terminates, `zzz` exits immediately:
```bash
zzz 1h --watch 12345
```

### Interactive Controls
While `zzz` is running in an interactive terminal, you can press:
- `Space` or `P`: Pause / Resume timer
- `+` or `=`: Add step duration (default: +30s)
- `-` or `_`: Subtract step duration (default: -30s)
- `S`: Skip timer and complete immediately
- `R`: Reset timer to start
- `Ctrl+C`: Cancel timer

Custom step duration:
```bash
zzz 10m --step 1m
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
| `--theme <THEME>` | `ZZZ_THEME` | Progress bar theme (`classic`, `cat`, `moon`, `pixel`, `matrix`) | `classic` |
| `--then / --exec <CMD>` | `ZZZ_THEN` | Execute command upon completion | — |
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
