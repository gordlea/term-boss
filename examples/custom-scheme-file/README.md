# Custom scheme file

Any Alacritty-format theme can live in `~/.config/term-boss/color_schemes/` and be selected
by file name:

```sh
mkdir -p ~/.config/term-boss/color_schemes
cp nord.toml ~/.config/term-boss/color_schemes/
```

Then `scheme = "nord"` in any `.term-boss.toml` picks it up. Files in that directory take
precedence over the library installed by `term-boss setup colorschemes`, so you can also
override a library scheme by saving a file with the same name.
