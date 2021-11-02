# unifont-bitmap

This crate incorporates the data for [GNU Unifont][1] in compressed binary
form. It concerns itself with glyph lookup and caching only. It does not
provide any rendering (like [`sdl2-unifont`][2] does) or even pixel lookup
(like [`unifont`][3] does). It is nothing more than a compression scheme
for the raw binary data represented in the `.hex` files that comprise
GNU Unifont's "source code".

[1]: http://unifoundry.com/unifont/index.html
[2]: https://crates.io/crates/sdl2-unifont
[3]: https://crates.io/crates/unifont

## Background

GNU Unifont is a bitmap font covering every character in Unicode. Narrow
characters are 8x16 pixels, and wide characters are 16x16 pixels. GNU
Unifont can be used to render any text that can be represented entirely
without combining characters, ligatures, or other frippery. For example, it
can render "ÿ", since that is encoded in Unicode as a single character:

1. `U+00FF LATIN SMALL LETTER Y WITH DIAERESIS` ("ÿ")

But it could *not* render "ÿ̰́", which is a sequence of:

1. `U+0079 LATIN SMALL LETTER Y` ("y")
2. `U+0308 COMBINING DIAERESIS` ("◌̈")
3. `U+0301 COMBINING ACUTE ACCENT` ("◌́")
4. `U+0330 COMBINING TILDE BELOW` ("◌̰")

In addition to basic concerns about putting pixels on the screen, any text
rendering system may also have to account for [bidirectional text][4] (and
right-to-left scripts in general) and take special care when [breaking
lines of text][5]. Not to mention "invisible characters". All of these
concerns are outside the scope of this crate, which, again, has the sole
and simple purpose of retrieving the individual GNU Unifont glyph that
represents a given Unicode code point.

[4]: https://unicode.org/reports/tr9/
[5]: https://unicode.org/reports/tr14/

The font data is embedded in your executable, in compressed form. The whole
thing is less than a megabyte in size when compressed, and if you somehow
end up using every page, it adds about 2.3 megabytes of runtime memory
overhead. This is a small price to pay for a font that covers every Unicode
character.

## Usage

Single-threaded usage is simple, via the [`Unifont`](struct.Unifont.html)
struct:

```rust
use unifont_bitmap::Unifont;
let mut unifont = Unifont::open();
// Get a bitmap, loading its page if necessary. Requires mut.
let my_bitmap = unifont.load_bitmap('井' as u32);
println!("{} pixels wide.", if my_bitmap.is_wide() { 16 } else { 8 });
println!("Bytes: {:?}", my_bitmap.get_bytes());
// Get a bitmap, iff its page is already loaded. Does not require mut.
let my_bitmap = unifont.get_bitmap('井' as u32).unwrap();
println!("{} pixels wide.", if my_bitmap.is_wide() { 16 } else { 8 });
println!("Bytes: {:?}", my_bitmap.get_bytes());
```

What you do from here is complicated, and outside this crate's pay grade.

## Legalese

The `unifont-bitmap` crate is copyright 2021, Solra Bizna, and licensed
under either of:

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

As for GNU Unifont:

> Copyright (C) 1998-2021 Roman Czyborra, Paul Hardy, Qianqian Fang,
> Andrew Miller, Johnnie Weaver, David Corbett, Nils Moskopp, Rebecca
> Bettencourt, et al. License: SIL Open Font License version 1.1 and
> GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>
> with the GNU Font Embedding Exception.

I believe that this license is compatible with `unifont-bitmap`'s use of
the font. If the font ends up statically linked into a non-GPL-compatible
application, e.g. for its own use in UI elements, my interpretation of the
license is that this is equivalent to embedding it into a document; thus
explicitly permitted by the Font Embedding Exception. If one of the
copyright holders and/or the Free Software Foundation disagrees with this
interpretation, I'd be open to discuss the issue.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the `unifont-bitmap` crate by you, as defined
in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
