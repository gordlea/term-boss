# Scheme precedence

| Directory | Result |
|---|---|
| `./` | `warm`: dark brown background, orange cursor, red and yellow palette slots |
| `cool/` | `cool` only: navy background, blue and cyan slots. No cursor colour, no red or yellow, because schemes never blend |
| `cool/tweaked/` | `cool`, except the background is `#000000` from the explicit `bgcolor` key |

Rules: the nearest `scheme` selection applies as a unit and farther selections are ignored;
explicit `fgcolor`, `bgcolor` and `cursorcolor` keys (or flags) override the scheme's values;
schemes can be defined at any level and are looked up across all of them.
