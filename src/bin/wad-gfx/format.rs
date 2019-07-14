use std::str::FromStr;

#[derive(Debug)]
pub enum Format {
    Indexed,
    Mask,
    Full,
}

impl FromStr for Format {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Format, &'static str> {
        match s {
            "indexed" => Ok(Format::Indexed),
            "i" => Ok(Format::Indexed),
            "mask" => Ok(Format::Mask),
            "m" => Ok(Format::Mask),
            "full" => Ok(Format::Full),
            "f" => Ok(Format::Full),
            _ => Err("format must be 'indexed'/'i', 'mask'/'m' or 'full'/'f'"),
        }
    }
}
