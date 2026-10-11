use std::{
    fmt::{self, Display, Formatter, LowerHex},
    str::FromStr,
};

use serde::Deserialize;
use strum::{IntoStaticStr, VariantArray};

pub type ColorValue = [u8; 3];

/// A colour given as a CSS name, `#rgb`/`#rrggbb` hex, Alacritty-style `0xrrggbb`, or
/// `rgb(r, g, b)`. Parsed from the CLI via `FromStr` and from TOML via `TryFrom<String>`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub enum ColorSpec {
    Named(Color),
    Rgb(ColorValue),
}

impl ColorSpec {
    pub const fn get_rgb(&self) -> ColorValue {
        match self {
            ColorSpec::Named(c) => c.get_rgb(),
            ColorSpec::Rgb(v) => *v,
        }
    }
}

const COLOR_ERR: &str = "expected a CSS colour name, #rgb, #rrggbb, 0xrrggbb or rgb(r, g, b)";

fn hex_byte(s: &str) -> Option<u8> {
    // `from_str_radix` would also accept a leading `+`.
    s.bytes().all(|b| b.is_ascii_hexdigit()).then(|| u8::from_str_radix(s, 16).ok())?
}

/// `rgb` or `rrggbb` hex digits (no prefix).
fn parse_hex(hex: &str) -> Option<ColorValue> {
    if !hex.is_ascii() {
        return None;
    }
    let parts = match hex.len() {
        3 => [0, 1, 2].map(|i| hex_byte(&hex[i..i + 1]).map(|n| n * 17)),
        6 => [0, 2, 4].map(|i| hex_byte(&hex[i..i + 2])),
        _ => return None,
    };
    match parts {
        [Some(r), Some(g), Some(b)] => Some([r, g, b]),
        _ => None,
    }
}

impl FromStr for ColorSpec {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix('#').or_else(|| s.strip_prefix("0x")).or_else(|| s.strip_prefix("0X")) {
            return parse_hex(hex).map(ColorSpec::Rgb).ok_or(COLOR_ERR);
        }
        if s.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("rgb(")) && s.ends_with(')') {
            let mut parts = s[4..s.len() - 1].split(',').map(|p| p.trim().parse::<u8>().ok());
            return match (parts.next(), parts.next(), parts.next(), parts.next()) {
                (Some(Some(r)), Some(Some(g)), Some(Some(b)), None) => Ok(ColorSpec::Rgb([r, g, b])),
                _ => Err(COLOR_ERR),
            };
        }
        Color::VARIANTS
            .iter()
            .find(|c| <&'static str>::from(*c).eq_ignore_ascii_case(s))
            .map(|c| ColorSpec::Named(*c))
            .ok_or(COLOR_ERR)
    }
}

impl TryFrom<String> for ColorSpec {
    type Error = &'static str;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// Human-readable form for diagnostics: `#rrggbb`, or `name (#rrggbb)` for named colours.
impl Display for ColorSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ColorSpec::Named(c) => write!(f, "{} (#{:x})", <&'static str>::from(c), self),
            ColorSpec::Rgb(_) => write!(f, "#{:x}", self),
        }
    }
}

impl LowerHex for ColorSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for byte in self.get_rgb() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// CSS named colours. Only the name table lives here; use `ColorSpec` everywhere else.
#[derive(Debug, Clone, Copy, IntoStaticStr, VariantArray)]
#[repr(u8)]
#[strum(serialize_all = "lowercase")]
pub enum Color {
    Antiquewhite,
    Aliceblue,
    Aqua,
    Aquamarine,
    Azure,
    Beige,
    Bisque,
    Black,
    Blanchedalmond,
    Blue,
    Blueviolet,
    Brown,
    Burlywood,
    Cadetblue,
    Chartreuse,
    Chocolate,
    Coral,
    Cornflowerblue,
    Cornsilk,
    Crimson,
    Cyan,
    Darkblue,
    Darkcyan,
    Darkgoldenrod,
    Darkgray,
    Darkgreen,
    Darkgrey,
    Darkkhaki,
    Darkmagenta,
    Darkolivegreen,
    Darkorange,
    Darkorchid,
    Darkred,
    Darksalmon,
    Darkseagreen,
    Darkslateblue,
    Darkslategray,
    Darkslategrey,
    Darkturquoise,
    Darkviolet,
    Deeppink,
    Deepskyblue,
    Dimgray,
    Dimgrey,
    Dodgerblue,
    Firebrick,
    Floralwhite,
    Forestgreen,
    Fuchsia,
    Gainsboro,
    Ghostwhite,
    Gold,
    Goldenrod,
    Gray,
    Green,
    Greenyellow,
    Grey,
    Honeydew,
    Hotpink,
    Indianred,
    Indigo,
    Ivory,
    Khaki,
    Lavender,
    Lavenderblush,
    Lawngreen,
    Lemonchiffon,
    Lightblue,
    Lightcoral,
    Lightcyan,
    Lightgoldenrodyellow,
    Lightgray,
    Lightgreen,
    Lightgrey,
    Lightpink,
    Lightsalmon,
    Lightseagreen,
    Lightskyblue,
    Lightslategray,
    Lightslategrey,
    Lightsteelblue,
    Lightyellow,
    Lime,
    Limegreen,
    Linen,
    Magenta,
    Maroon,
    Mediumaquamarine,
    Mediumblue,
    Mediumorchid,
    Mediumpurple,
    Mediumseagreen,
    Mediumslateblue,
    Mediumspringgreen,
    Mediumturquoise,
    Mediumvioletred,
    Midnightblue,
    Mintcream,
    Mistyrose,
    Moccasin,
    Navajowhite,
    Navy,
    Oldlace,
    Olive,
    Olivedrab,
    Orange,
    Orangered,
    Orchid,
    Palegoldenrod,
    Palegreen,
    Paleturquoise,
    Palevioletred,
    Papayawhip,
    Peachpuff,
    Peru,
    Pink,
    Plum,
    Powderblue,
    Purple,
    Rebeccapurple,
    Red,
    Rosybrown,
    Royalblue,
    Saddlebrown,
    Salmon,
    Sandybrown,
    Seagreen,
    Seashell,
    Sienna,
    Silver,
    Skyblue,
    Slateblue,
    Slategray,
    Slategrey,
    Snow,
    Springgreen,
    Steelblue,
    Tan,
    Teal,
    Thistle,
    Tomato,
    Turquoise,
    Violet,
    Wheat,
    White,
    Whitesmoke,
    Yellow,
    Yellowgreen,
}

impl Color {
    pub const fn get_rgb(&self) -> ColorValue {
        match self {
            Color::Antiquewhite => [250, 235, 215],
            Color::Aliceblue => [240, 248, 255],
            Color::Aqua => [0, 255, 255],
            Color::Aquamarine => [127, 255, 212],
            Color::Azure => [240, 255, 255],
            Color::Beige => [245, 245, 220],
            Color::Bisque => [255, 228, 196],
            Color::Black => [0, 0, 0],
            Color::Blanchedalmond => [255, 235, 205],
            Color::Blue => [0, 0, 255],
            Color::Blueviolet => [138, 43, 226],
            Color::Brown => [165, 42, 42],
            Color::Burlywood => [222, 184, 135],
            Color::Cadetblue => [95, 158, 160],
            Color::Chartreuse => [127, 255, 0],
            Color::Chocolate => [210, 105, 30],
            Color::Coral => [255, 127, 80],
            Color::Cornflowerblue => [100, 149, 237],
            Color::Cornsilk => [255, 248, 220],
            Color::Crimson => [220, 20, 60],
            Color::Cyan => [0, 255, 255],
            Color::Darkblue => [0, 0, 139],
            Color::Darkcyan => [0, 139, 139],
            Color::Darkgoldenrod => [184, 134, 11],
            Color::Darkgray => [169, 169, 169],
            Color::Darkgreen => [0, 100, 0],
            Color::Darkgrey => [169, 169, 169],
            Color::Darkkhaki => [189, 183, 107],
            Color::Darkmagenta => [139, 0, 139],
            Color::Darkolivegreen => [85, 107, 47],
            Color::Darkorange => [255, 140, 0],
            Color::Darkorchid => [153, 50, 204],
            Color::Darkred => [139, 0, 0],
            Color::Darksalmon => [233, 150, 122],
            Color::Darkseagreen => [143, 188, 143],
            Color::Darkslateblue => [72, 61, 139],
            Color::Darkslategray => [47, 79, 79],
            Color::Darkslategrey => [47, 79, 79],
            Color::Darkturquoise => [0, 206, 209],
            Color::Darkviolet => [148, 0, 211],
            Color::Deeppink => [255, 20, 147],
            Color::Deepskyblue => [0, 191, 255],
            Color::Dimgray => [105, 105, 105],
            Color::Dimgrey => [105, 105, 105],
            Color::Dodgerblue => [30, 144, 255],
            Color::Firebrick => [178, 34, 34],
            Color::Floralwhite => [255, 250, 240],
            Color::Forestgreen => [34, 139, 34],
            Color::Fuchsia => [255, 0, 255],
            Color::Gainsboro => [220, 220, 220],
            Color::Ghostwhite => [248, 248, 255],
            Color::Gold => [255, 215, 0],
            Color::Goldenrod => [218, 165, 32],
            Color::Gray => [128, 128, 128],
            Color::Green => [0, 128, 0],
            Color::Greenyellow => [173, 255, 47],
            Color::Grey => [128, 128, 128],
            Color::Honeydew => [240, 255, 240],
            Color::Hotpink => [255, 105, 180],
            Color::Indianred => [205, 92, 92],
            Color::Indigo => [75, 0, 130],
            Color::Ivory => [255, 255, 240],
            Color::Khaki => [240, 230, 140],
            Color::Lavender => [230, 230, 250],
            Color::Lavenderblush => [255, 240, 245],
            Color::Lawngreen => [124, 252, 0],
            Color::Lemonchiffon => [255, 250, 205],
            Color::Lightblue => [173, 216, 230],
            Color::Lightcoral => [240, 128, 128],
            Color::Lightcyan => [224, 255, 255],
            Color::Lightgoldenrodyellow => [250, 250, 210],
            Color::Lightgray => [211, 211, 211],
            Color::Lightgreen => [144, 238, 144],
            Color::Lightgrey => [211, 211, 211],
            Color::Lightpink => [255, 182, 193],
            Color::Lightsalmon => [255, 160, 122],
            Color::Lightseagreen => [32, 178, 170],
            Color::Lightskyblue => [135, 206, 250],
            Color::Lightslategray => [119, 136, 153],
            Color::Lightslategrey => [119, 136, 153],
            Color::Lightsteelblue => [176, 196, 222],
            Color::Lightyellow => [255, 255, 224],
            Color::Lime => [0, 255, 0],
            Color::Limegreen => [50, 205, 50],
            Color::Linen => [250, 240, 230],
            Color::Magenta => [255, 0, 255],
            Color::Maroon => [128, 0, 0],
            Color::Mediumaquamarine => [102, 205, 170],
            Color::Mediumblue => [0, 0, 205],
            Color::Mediumorchid => [186, 85, 211],
            Color::Mediumpurple => [147, 112, 219],
            Color::Mediumseagreen => [60, 179, 113],
            Color::Mediumslateblue => [123, 104, 238],
            Color::Mediumspringgreen => [0, 250, 154],
            Color::Mediumturquoise => [72, 209, 204],
            Color::Mediumvioletred => [199, 21, 133],
            Color::Midnightblue => [25, 25, 112],
            Color::Mintcream => [245, 255, 250],
            Color::Mistyrose => [255, 228, 225],
            Color::Moccasin => [255, 228, 181],
            Color::Navajowhite => [255, 222, 173],
            Color::Navy => [0, 0, 128],
            Color::Oldlace => [253, 245, 230],
            Color::Olive => [128, 128, 0],
            Color::Olivedrab => [107, 142, 35],
            Color::Orange => [255, 165, 0],
            Color::Orangered => [255, 69, 0],
            Color::Orchid => [218, 112, 214],
            Color::Palegoldenrod => [238, 232, 170],
            Color::Palegreen => [152, 251, 152],
            Color::Paleturquoise => [175, 238, 238],
            Color::Palevioletred => [219, 112, 147],
            Color::Papayawhip => [255, 239, 213],
            Color::Peachpuff => [255, 218, 185],
            Color::Peru => [205, 133, 63],
            Color::Pink => [255, 192, 203],
            Color::Plum => [221, 160, 221],
            Color::Powderblue => [176, 224, 230],
            Color::Purple => [128, 0, 128],
            Color::Rebeccapurple => [102, 51, 153],
            Color::Red => [255, 0, 0],
            Color::Rosybrown => [188, 143, 143],
            Color::Royalblue => [65, 105, 225],
            Color::Saddlebrown => [139, 69, 19],
            Color::Salmon => [250, 128, 114],
            Color::Sandybrown => [244, 164, 96],
            Color::Seagreen => [46, 139, 87],
            Color::Seashell => [255, 245, 238],
            Color::Sienna => [160, 82, 45],
            Color::Silver => [192, 192, 192],
            Color::Skyblue => [135, 206, 235],
            Color::Slateblue => [106, 90, 205],
            Color::Slategray => [112, 128, 144],
            Color::Slategrey => [112, 128, 144],
            Color::Snow => [255, 250, 250],
            Color::Springgreen => [0, 255, 127],
            Color::Steelblue => [70, 130, 180],
            Color::Tan => [210, 180, 140],
            Color::Teal => [0, 128, 128],
            Color::Thistle => [216, 191, 216],
            Color::Tomato => [255, 99, 71],
            Color::Turquoise => [64, 224, 208],
            Color::Violet => [238, 130, 238],
            Color::Wheat => [245, 222, 179],
            Color::White => [255, 255, 255],
            Color::Whitesmoke => [245, 245, 245],
            Color::Yellow => [255, 255, 0],
            Color::Yellowgreen => [154, 205, 50],
        }
    }
}

