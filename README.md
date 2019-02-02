Extract graphics assets from doom.wad. At this point only flats are supported.

`wad-gfx` takes two positional arguments. First, the path to `doom.wad`,
second the name of a flat to extract. A command line invocation may look like
this:

    wad-gfx doom.wad FLOOR5_1

The output filename is implicitly generated from the flat name by converting
it to lower case and appending `.png`. For this example, `wad-gfx` would
write the image to a file named `floor5_1.png`. The flat name is case
sensitive. To see the list of all flats available, use [`wad-ls`] and look for
all entries listed between `F_START` and `F_END`.

There are a few command line options available for configuring the extraction:

Palettes: The original game includes 14 palettes for different full-screen
effects, including the red coloring when you get hurt. Palette 0 is normal.

Colormap: In order to fade images to different brightness, 32 different
colormaps are used. Colormap 0 is the brightest. Additionally, colormap 32 is
used for god mode and 33 is all-black.

Scale: Because screen resolutions have increased many-fold since Doom was
released, the graphic assets are woefully small. Use the scale option to
embiggen the pixels using beautiful nearest neighbor filtering.

[`wad-ls`]: https://github.com/maghoff/wad/

Try it out
----------
Install via Rust toolchain:

    cargo install wad-gfx

Run:

    wad-gfx doom.wad FLOOR5_1
    display floor5_1.png

Command line options
--------------------

    wad-gfx 0.1.0
    Magnus Hovland Hoff <maghoff@gmail.com>
    Extract graphics from Doom WAD files

    USAGE:
        wad-gfx [OPTIONS] <input> <flat>

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -c, --colormap <colormap>    Which colormap to use (0-33) [default: 0]
        -p, --palette <palette>      Which palette to use (0-13) [default: 0]
        -s, --scale <scale>          Scale with beautiful nearest neighbor filtering [default: 2]

    ARGS:
        <input>    Input WAD file
        <flat>     Flat to extract
