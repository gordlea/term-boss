# Parent chain

`${parent}` in `tabname` or `badgetext` is the same field from the next config above that sets
it, already expanded. From `api/`:

| Key | Value | Comes from |
|---|---|---|
| `badgetext` | `api · team <host>` | `api/` extends the root's `team ${host}` |
| `tabname` | `parent-chain` | root (not set in `api/`, so inherited as usual) |

The chain continues upward through every level, including the user and host-wide base
configs, so `badgetext = "someuser on ${parent}"` in `~/.config/term-boss/user-base-config.toml`
would be picked up by a root config that itself used `${parent}`. `term-boss show` prints each
link of the chain under "Resolved".
