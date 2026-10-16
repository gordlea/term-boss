# Variables in tab names and badges

`tabname` and `badgetext` expand `${…}` references when `term-boss set` runs. Lowercase names
are built-ins that term-boss resolves itself, so they work the same from the shell hook, from a
manual `term-boss set`, and under any shell; everything else is read from the environment.
With `AWS_PROFILE=prod` exported, `term-boss show` here ends with:

```
Built-in variables:
  ${hostname} = "megatron"
  ${host} = "megatron"
  ${username} = "me"
  ${pwd} = "/home/me/dev/iterm-boss/examples/env-vars"
  ${pwd_short} = "~/dev/iterm-boss/examples/env-vars"
  ${dir} = "env-vars"
  ${tty} = "/dev/pts/3"
  ${git_branch} = "main"
  ${repo} = "iterm-boss"
  ${terminal} = "Wezterm"

Resolved:
  tabname = "${repo:-${dir}}${git_branch:+ (${git_branch})}"
  badgetext = "${username}@${host} ${AWS_PROFILE:-no aws profile}"
  ...
  tabname expands to "iterm-boss (main)"
  badgetext expands to "me@megatron prod"
```

Outside a git repository `${repo}` is unset, so the tab falls back to the directory name and
the ` (branch)` part disappears. Unset `AWS_PROFILE` and the badge shows `no aws profile`. A
value that expands to nothing at all (`"${AWS_PROFILE}"` with no profile) is treated as unset,
so the terminal never shows an empty badge.

The default shell hook runs on `cd`, so an `export` or a branch switch changes the tab on the
next directory change. For a tab that follows them immediately, install the hook as
`eval "$(term-boss env zsh --hook precmd)"`, which runs `term-boss set` before every prompt.
