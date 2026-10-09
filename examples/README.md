# Examples

Each directory is a self-contained `.term-boss.toml` setup. With the shell hook installed,
`cd` into one and your terminal changes. To see how a directory resolves without changing
anything, run `term-boss show` inside it. It lists the config files found, the keys each sets,
the selected scheme and the final values:

```sh
cd examples/nested-configs/backend/prod
term-boss show
```

To see the raw escape sequences `set` would emit, pipe it through `cat -v`:

```sh
TERM_PROGRAM=iTerm.app term-boss set | cat -v
```

| Example | Shows |
|---|---|
| [`standard-config/`](standard-config) | Every top-level key in one file |
| [`nested-configs/`](nested-configs) | Field-by-field inheritance from parent directories |
| [`inline-scheme/`](inline-scheme) | A colour scheme defined and selected in the same file |
| [`library-scheme/`](library-scheme) | Selecting one of WezTerm's built-in schemes by name |
| [`custom-scheme-file/`](custom-scheme-file) | A standalone Alacritty-format scheme file in `~/.config/term-boss/color_schemes/` |
| [`scheme-precedence/`](scheme-precedence) | Nearest scheme wins outright; explicit keys override scheme values |
| [`home-reset/`](home-reset) | A `$HOME` config that restores defaults when you leave a themed project |

Configs are resolved from the current directory up to `/`, so these examples are only
affected by any `.term-boss.toml` you have in a parent of this repository.
