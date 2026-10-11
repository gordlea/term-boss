# term-boss

Per-directory terminal theming. Drop a `.term-boss.toml` in a project and your tab colour,
tab title, badge, pane colours and ANSI palette change whenever you `cd` into it.

Works with iTerm2, WezTerm, Alacritty and Konsole. Settings are applied with standard
escape sequences, so nothing needs to be running in the background.

## Install

```sh
cargo install --path .
```

### Shell hook

`term-boss env zsh` prints a `chpwd` hook that runs `term-boss set` on every directory change.
Add it to `~/.zshrc`, and call `term-boss set` once so a new shell picks up the current
directory's settings:

```sh
eval "$(term-boss env zsh)"
term-boss set
```

### WezTerm

WezTerm has no escape sequence for tab colours or badges, so term-boss ships a small Lua
module that reads per-pane user variables. Install it once:

```sh
term-boss setup wezterm
```

This writes `termboss.lua` into your WezTerm config directory and tells you the one line to
add to `wezterm.lua`:

```lua
require('termboss').apply_to_config(config)
```

The module registers `format-tab-title` and `update-status` handlers. WezTerm only runs the
first handler registered for an event, so remove any existing `format-tab-title` handler.

WezTerm reloads its configuration automatically only in windows that already had a config file
when they started. If `~/.config/wezterm/wezterm.lua` did not exist before you ran the setup,
press `Ctrl+Shift+R` in each open window or restart WezTerm. `term-boss show` reports whether
the module is installed and required.

### Colour scheme library

Optionally fetch the roughly 1000 colour schemes built into
[WezTerm](https://github.com/wezterm/wezterm), which WezTerm itself assembles from
iTerm2-Color-Schemes, Gogh, base16 and terminal.sexy:

```sh
term-boss setup colorschemes
```

This explains what it is about to do, asks for confirmation, downloads WezTerm's published
scheme data (one JSON file, under 1 MB) with `curl`, and converts every scheme into an
Alacritty-format file under `~/.config/term-boss/wezterm_schemes/`. Rerun the command to
update, or pass `--yes` to skip the prompt. The names are exactly WezTerm's `color_scheme`
names, so one string works in both `wezterm.lua` and `.term-boss.toml`:

```toml
scheme = "Dracula (Official)"
```

Requires `curl`. This is the only command that touches the network.

## Usage

```
term-boss set [OPTIONS]
```

| Flag | Env var | Effect |
|---|---|---|
| `--tabname <text>` | `TERM_BOSS_NAME` | Tab title |
| `--tabcolor <colour>` | `TERM_BOSS_BG_COLOR` | Tab background colour |
| `--badgetext <text>` | `TERM_BOSS_BADGETEXT` | iTerm2 badge, WezTerm right-status text, Konsole badge; elsewhere appended to the tab title as `name [badge]` |
| `--fgcolor <colour>` | `TERM_BOSS_PANE_FG` | Pane foreground (OSC 10) |
| `--bgcolor <colour>` | `TERM_BOSS_PANE_BG` | Pane background (OSC 11) |
| `--cursorcolor <colour>` | `TERM_BOSS_PANE_CURSOR` | Cursor colour (OSC 12) |
| `--scheme <name>` | `TERM_BOSS_SCHEME` | Apply a colour scheme, inline or from `~/.config/term-boss/color_schemes/` |
| `--reset` | `TERM_BOSS_RESET` | Reset anything not explicitly set back to the terminal default |

Colours accept a CSS colour name (`tomato`), `#rgb`, `#rrggbb`, or `rgb(r, g, b)`.

`term-boss show` takes the same flags and explains what `set` would do from the current
directory without touching the terminal: the detected terminal, every config file found, the
keys each one sets, which scheme was selected and where it came from, and the final values.
Reach for it whenever a `cd` does not seem to change anything.

```
$ term-boss show
Terminal: Wezterm
  TERM_PROGRAM=WezTerm
  ...

Config files (nearest first):
  /home/me/dev/api/backend/.term-boss.toml
  /home/me/dev/api/.term-boss.toml

CLI / env:
  (no keys)

/home/me/dev/api/backend/.term-boss.toml:
  tabname = "backend"

/home/me/dev/api/.term-boss.toml:
  scheme = "repo"
  tabcolor = #2b6cb0
  reset = true
  color_schemes: repo

Scheme: repo (inline, defined in /home/me/dev/api/.term-boss.toml)

Resolved:
  tabname = "backend"
  ...
  palette = 16 slots
```

On iTerm2 the badge text supports iTerm's variables when single-quoted, for example
`--badgetext '\(hostname)'`. See <https://iterm2.com/documentation-variables.html>.

## Config files

`term-boss set` looks for `.term-boss.toml` in the current directory and every parent up
to `/`. Every key accepts the same values as the matching CLI flag.
Runnable examples of every use case live in [`examples/`](examples). `term-boss show` prints
which file each value came from.

```toml
tabname = "api"
tabcolor = "#2b6cb0"
badgetext = "prod ⚠"
fgcolor = "#eeeeee"
bgcolor = "#1a202c"
cursorcolor = "orange"
reset = true
```

### Resolution order

Nearer definitions win, field by field:

1. CLI flags and environment variables
2. `.term-boss.toml` in the current directory
3. `.term-boss.toml` in each parent directory, nearest first

So a repo-level config can set a tab colour while a subdirectory overrides only the tab name.

Unknown keys are an error, so typos fail loudly with the file, line and column.

## Colour schemes

Colour schemes use [Alacritty's theme format](https://github.com/alacritty/alacritty-theme), so
any file from that repository works unchanged. A scheme sets the pane foreground and
background, the cursor colour and the 16 ANSI palette slots. Schemes are colours only: tab
colour, tab name and badge are set per directory, not by a scheme.

Select a scheme with `scheme = "<name>"` in a config file or `--scheme <name>` on the CLI.
The name is resolved in this order:

1. An inline definition under `[color_schemes.<name>]` in any `.term-boss.toml` in the
   hierarchy, nearest first.
2. The file `$XDG_CONFIG_HOME/term-boss/color_schemes/<name>.toml`, defaulting to
   `~/.config/term-boss/color_schemes/<name>.toml`.
3. `<name>.toml` in the library written by `term-boss setup colorschemes`, if present. Names
   are WezTerm's scheme names, spaces and all: `"Dracula (Official)"`, `"Catppuccin Mocha"`,
   `"Solarized Dark (base16)"`.

### Scheme files

```sh
mkdir -p ~/.config/term-boss/color_schemes
curl -o ~/.config/term-boss/color_schemes/solarized_dark.toml \
  https://raw.githubusercontent.com/alacritty/alacritty-theme/master/themes/solarized_dark.toml
```

Then in a project:

```toml
scheme = "solarized_dark"
tabcolor = "#002b36"
```

A scheme file looks like this. Colours may be `#rrggbb` or Alacritty's `0xrrggbb`, and the
term-boss formats (`#rgb`, CSS names, `rgb(r, g, b)`) are accepted too.

```toml
[colors.primary]
background = "0x2E3440"
foreground = "0xD8DEE9"

[colors.cursor]
cursor = "#D8DEE9"

[colors.normal]
black   = "0x3B4252"
red     = "0xBF616A"
green   = "0xA3BE8C"
yellow  = "0xEBCB8B"
blue    = "0x81A1C1"
magenta = "0xB48EAD"
cyan    = "0x88C0D0"
white   = "0xE5E9F0"

[colors.bright]
black   = "0x4C566A"
red     = "0xBF616A"
green   = "0xA3BE8C"
yellow  = "0xEBCB8B"
blue    = "0x81A1C1"
magenta = "0xB48EAD"
cyan    = "0x8FBCBB"
white   = "0xECEFF4"
```

### Inline schemes

The same shape works inside `.term-boss.toml`, under `[color_schemes.<name>]`. To paste a theme
file in, prefix each of its table headers with `color_schemes.<name>.`.

```toml
scheme = "danger"

[color_schemes.danger.colors.primary]
background = "#1a0000"

[color_schemes.danger.colors.normal]
red = "#ff5555"

[color_schemes.danger.colors.bright]
red = "#ff7979"
```

Any subset of the colours may be given; the terminal keeps its own value for the rest.

### What is applied

| Alacritty section | Effect |
|---|---|
| `colors.primary.foreground` | Pane foreground (OSC 10) |
| `colors.primary.background` | Pane background (OSC 11) |
| `colors.cursor.cursor` | Cursor colour (OSC 12) |
| `colors.normal.*` | Palette slots 0 to 7 (OSC 4) |
| `colors.bright.*` | Palette slots 8 to 15 (OSC 4) |

Everything else a theme file may contain (`dim`, `selection`, `search`, `hints`,
`vi_mode_cursor`, `footer_bar`, `line_indicator`, `indexed_colors`,
`draw_bold_text_with_bright_colors`, `transparent_background_colors`) is accepted and
ignored. A cursor colour of `CellForeground` or `CellBackground` has no escape sequence
and is skipped. Misspelt keys inside a section term-boss reads are an error.

### Rules

- Only the nearest `scheme` selection applies. If the CLI or a child directory picks one
  scheme and a parent picks another, the parent's scheme is ignored entirely so two schemes
  never blend.
- Explicit `fgcolor`, `bgcolor` and `cursorcolor` keys or flags override the scheme's values.
- Selecting an unknown scheme is an error that lists the known names (capped at 20) and, if
  the library is not installed, how to fetch it.

### `--reset`

With `reset = true` (or `--reset`), anything not set after resolution is returned to the
terminal's default: tab colour and name, badge, pane colours (OSC 110/111/112) and any
palette slots the scheme did not cover (OSC 104). Put it in a config near the top of your
tree so leaving a themed project clears its colours.

## Security notes

`.term-boss.toml` files are read from every directory you `cd` into, including cloned
repositories, so they are treated as untrusted:

- `term-boss set` never touches the network. Schemes come only from local files, and the
  library is fetched solely by the explicit `setup colorschemes` command, from a fixed URL over
  HTTPS with size and time limits.
- Colour schemes are colours only. Values are parsed into RGB bytes before anything is emitted,
  so a scheme cannot inject escape sequences or set anything but colours. The downloaded library
  is converted colour by colour, so the generated files contain only validated values.
- Tab names and badge text have control characters stripped before they are sent to the
  terminal, so a config cannot terminate the title sequence early and smuggle in another one.
- Unknown keys anywhere in a config or scheme are errors rather than silently ignored.

## Terminal support

Legend: **✓** native support · **title** shown in the tab/window title instead · **–** not
available · **n/a** the terminal has no such thing.

| Feature | iTerm2 | WezTerm | Alacritty | Konsole | Other* |
|---|---|---|---|---|---|
| Detected by | `TERM_PROGRAM=iTerm.app` or `ITERM_SESSION_ID` | `TERM_PROGRAM=WezTerm` or `WEZTERM_PANE` | `ALACRITTY_WINDOW_ID`, `ALACRITTY_SOCKET` or `TERM` containing `alacritty` | `KONSOLE_VERSION` | any other `TERM` except `dumb` |
| Tab colour | ✓ OSC 6 | ✓ user var + Lua module | n/a (no tabs) | ✓ OSC 34 | – |
| Tab name | ✓ OSC 0 | ✓ `wezterm cli set-tab-title` | ✓ OSC 0 (window title) | ✓ OSC 30 | ✓ OSC 0 (window title) |
| Badge text | ✓ OSC 1337 badge | ✓ user var, right status | title | ✓ D-Bus badge overlay; title if D-Bus is unavailable | title |
| Pane foreground / background / cursor | ✓ OSC 10 / 11 / 12 | ✓ | ✓ | ✓ | ✓ |
| 16-colour ANSI palette | ✓ OSC 4 | ✓ | ✓ | ✓ | ✓ |
| `reset` of the above | ✓ | ✓ | ✓ | ✓ (tab colour via D-Bus; title → `%d : %n`) | ✓ |
| Extra setup | – | `term-boss setup wezterm` once | – | `dbus-send` or `busctl` on PATH for badge and tab-colour reset | – |
| Verified on | not re-tested here (original target) | 20261005 | 0.17 | 26.08 | xterm 411 |

\* Any unrecognised terminal (xterm, kitty, foot, GNOME Terminal, Ghostty, Windows Terminal,
…) gets the standard xterm escapes: OSC 0 for the title, OSC 10/11/12 for pane colours and
OSC 4 for the palette. When `TERM` is unset or `dumb` (scripts, Emacs shells, CI),
`term-boss set` exits silently without emitting anything.

Where a terminal has tabs or a window title but no badge, the badge is appended to the title as
`name [badge]`, or shown alone as `[badge]` when no tab name is set. `term-boss show` prints
which terminal was detected, the evidence it used, and this fallback when it applies.

Detection checks `TERM_PROGRAM` first, then the terminal-specific variables, then `TERM`.
Alacritty is recognised even when its config sets `TERM` to `xterm-256color`.

Konsole notes: the tab title uses Konsole's own OSC 30, so it shows regardless of the profile's
title format, and resetting it restores Konsole's default format `%d : %n`. Konsole expands
`%` tokens in titles, so avoid `%` in a tab name. The badge and clearing the tab colour go
through Konsole's session D-Bus interface, which Konsole advertises to the shell as
`KONSOLE_DBUS_SERVICE` and `KONSOLE_DBUS_SESSION`. Older Konsole releases ignore the sequences
they do not know.

## Development

```sh
cargo build --release
TERM_PROGRAM=iTerm.app ./target/release/term-boss set --tabcolor tomato | cat -v
```

Piping through `cat -v` shows the raw escape sequences instead of applying them.
