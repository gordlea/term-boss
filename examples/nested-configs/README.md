# Nested configs

Resolution is field by field, nearest directory first. From `backend/prod/`:

| Key | Value | Comes from |
|---|---|---|
| `tabname` | `backend` | `backend/` |
| `tabcolor` | `crimson` | `backend/prod/` |
| `badgetext` | `⚠ PROD` | `backend/prod/` |
| `scheme` | `repo` | root |
| `reset` | `true` | root |

From `frontend/` only `tabname` differs from the root. The inline scheme is defined once at the
root and applies everywhere beneath it.
