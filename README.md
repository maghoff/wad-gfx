Extract graphics assets from doom.wad. Support currently includes [flats] and
[sprites]. Support for textures is coming.

[flats]: https://magnushoff.com/blog/flats/
[sprites]: https://magnushoff.com/blog/sprites/

`wad-gfx` takes three positional arguments. First, the path to `doom.wad`,
second the name of a lump to extract and last, the datatype of the lump. A
command line invocation may look like this:

    wad-gfx doom.wad floor5_1 flat

This invocation will make `wad-gfx` extract the lump floor5_1 as a flat (which
it is) and save it to floor5_1.png.

⚠ The command line interface is subject to change.

Flats and sprites have many differences, and consequently have different
command line options. There are a few options in common as well.

Common command line options
===========================
    wad-gfx 0.1.3
    Magnus Hovland Hoff <maghoff@gmail.com>
    Extract graphics from Doom WAD files

    USAGE:
        wad-gfx [OPTIONS] <input> <name> <SUBCOMMAND>

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -c, --colormap <colormap>    Which colormap to use (0-33) [default: 0]
        -o, --output <output>        Output filename. If absent, will default to <name>.png
        -p, --palette <palette>      Which palette to use (0-13) [default: 0]
        -s, --scale <scale>          Scale with beautiful nearest neighbor filtering [default: 2]

    ARGS:
        <input>    Input WAD file
        <name>     The lump name of the graphic to extract

    SUBCOMMANDS:
        flat      Extract a flat
        help      Prints this message or the help of the given subcommand(s)
        sprite    Extract a sprite

Palettes: The original game includes 14 palettes for different full-screen
effects, including the red coloring when you get hurt. Palette 0 is normal.

Colormap: In order to fade images to different brightness, 32 different
colormaps are used. Colormap 0 is the brightest. Additionally, colormap 32 is
used for god mode and 33 is all-black.

Scale: Because screen resolutions have increased many-fold since Doom was
released, the graphic assets are woefully small. Use the scale option to
embiggen the pixels using beautiful nearest neighbor filtering.

Sprites
=======
    FLAGS:
        -a, --anamorphic    Output anamorphic (non-square) pixels. Like the
                            original assets, the pixel aspect ratio will be 5:6.
        -h, --help          Prints help information
        -I, --info          Print information about the sprite to stdout instead
                            of generating an output image
        -V, --version       Prints version information

    OPTIONS:
        -b, --background <background>   Color index to use for the background
            --canvas <canvas_size>      Canvas size for the output. Defaults to
                                        the size of the sprite. See the output
                                        from --info.
        -f, --format <format>           Output format: full/f, indexed/i or mask/m.
                                        Full color uses the alpha channel for
                                        transparency. Indexed color does not include
                                        transparency, but can be combined with
                                        the mask for transparent sprites. [default: full]
            --pos <pos>                 Place the sprite's hotspot at these
                                        coordinates. Defaults to the coordinates of
                                        the hotspot. See the output from --info.

Example invocation:

    wad-gfx doom.wad trooa1 sprite
