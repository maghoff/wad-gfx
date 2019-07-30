use std::path::Path;
use structopt::StructOpt;
use wad::EntryId;
use wad_gfx::{parse_pnames, render_texture, LazyPatchProvider, TextureDirectory};

use crate::format::Format;
use crate::sprite::{sprite_cmd, SpriteOpt};

#[derive(Debug, StructOpt)]
pub struct ExtractOpt {
    /// The name of the texture to extract
    name: String,

    /// Print information about the texture in DeuTex format to stdout instead
    /// of generating an output image
    #[structopt(short = "I", long = "info")]
    info: bool,

    /// Output format: full/f, indexed/i or mask/m. Full color uses the
    /// alpha channel for transparency. Indexed color does not include
    /// transparency, but can be combined with the mask for transparent
    /// sprites.
    #[structopt(short = "f", long = "format", default_value = "full")]
    format: Format,

    /// Color index to use for the background
    #[structopt(short = "b", long = "background")]
    background: Option<u8>,

    /// Output anamorphic (non-square) pixels. Like the original assets,
    /// the pixel aspect ratio will be 5:6.
    #[structopt(short = "a", long = "anamorphic")]
    anamorphic: bool,
}

#[derive(Debug, StructOpt)]
pub enum TextureOpt {
    /// List textures in directory
    #[structopt(name = "list")]
    List,

    /// Extract a texture
    #[structopt(name = "extract")]
    Extract(ExtractOpt),
}

pub fn texture_cmd(
    wad: &wad::Wad,
    palette: &[u8],
    colormap: &[u8],
    texture_dir: &[u8],
    scale: usize,
    output: impl AsRef<Path>,
    opt: TextureOpt,
) -> Result<(), Box<dyn std::error::Error>> {
    let texture_dir = TextureDirectory::new(texture_dir);

    match opt {
        TextureOpt::List => {
            for i in 0..texture_dir.len() {
                let texture = texture_dir.texture(i);
                println!("{}", EntryId::from_bytes(&texture.name()));
            }

            Ok(())
        }
        TextureOpt::Extract(opt) => {
            let pnames = parse_pnames(wad.by_id(b"PNAMES").ok_or("Missing PNAMES")?);

            let texture_id = EntryId::from_str(&opt.name)
                .ok_or_else(|| format!("Invalid ID: {:?}", opt.name))?;

            let mut texture = None;

            for i in 0..texture_dir.len() {
                let t = texture_dir.texture(i);
                if EntryId::from_bytes(&t.name()) == texture_id {
                    texture = Some(t);
                    break;
                }
            }

            let texture = texture.ok_or_else(|| format!("Unable to find texture {}", opt.name))?;

            if opt.info {
                println!("; TextureName Width Height");
                println!("{} {} {}", texture_id, texture.width(), texture.height());
                println!("; PatchName Xoffset Yoffset");
                for i in 0..texture.len() {
                    let patch = texture.patch(i);
                    println!(
                        "* {} {} {}",
                        EntryId::from_bytes(&pnames[patch.patch_id as usize]),
                        patch.origin_x,
                        patch.origin_y
                    );
                }
                return Ok(());
            }

            let patch_provider = LazyPatchProvider::new(wad.as_slice(), pnames);

            let texture_sprite = render_texture(texture, &patch_provider);

            // TODO Refactor to avoid reusing top-level entrypoint
            sprite_cmd(
                palette,
                colormap,
                &texture_sprite,
                scale,
                output,
                SpriteOpt {
                    canvas_size: None,
                    pos: None,
                    info: false,
                    format: opt.format,
                    background: opt.background,
                    anamorphic: opt.anamorphic,
                },
            )
        }
    }
}
