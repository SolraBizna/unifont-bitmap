//! This crate incorporates the data for [GNU Unifont][1] in compressed binary
//! form. It concerns itself with glyph lookup and caching only. It does not
//! provide any rendering (like [`sdl2-unifont`][2] does) or even pixel lookup
//! (like [`unifont`][3] does). It is nothing more than a compression scheme
//! for the raw binary data represented in the `.hex` files that comprise
//! GNU Unifont's "source code".
//!
//! [1]: http://unifoundry.com/unifont/index.html
//! [2]: https://crates.io/crates/sdl2-unifont
//! [3]: https://crates.io/crates/unifont
//!
//! # Background
//!
//! GNU Unifont is a bitmap font covering every character in Unicode. Narrow
//! characters are 8x16 pixels, and wide characters are 16x16 pixels. GNU
//! Unifont can be used to render any text that can be represented entirely
//! without combining characters, ligatures, or other frippery. For example, it
//! can render "ÿ", since that is encoded in Unicode as a single character:
//!
//! 1. `U+00FF LATIN SMALL LETTER Y WITH DIAERESIS` ("ÿ")
//!
//! But it could *not* render "ÿ̰́", which is a sequence of:
//!
//! 1. `U+0079 LATIN SMALL LETTER Y` ("y")
//! 2. `U+0308 COMBINING DIAERESIS` ("◌̈")
//! 3. `U+0301 COMBINING ACUTE ACCENT` ("◌́")
//! 4. `U+0330 COMBINING TILDE BELOW` ("◌̰")
//!
//! In addition to basic concerns about putting pixels on the screen, any text
//! rendering system may also have to account for [bidirectional text][4] (and
//! right-to-left scripts in general) and take special care when [breaking
//! lines of text][5]. Not to mention "invisible characters". All of these
//! concerns are outside the scope of this crate, which, again, has the sole
//! and simple purpose of retrieving the individual GNU Unifont glyph that
//! represents a given Unicode code point.
//!
//! [4]: https://unicode.org/reports/tr9/
//! [5]: https://unicode.org/reports/tr14/
//!
//! The font data is embedded in your executable, in compressed form. The whole
//! thing is less than a megabyte in size when compressed, and if you somehow
//! end up using every page, it adds about 2.3 megabytes of runtime memory
//! overhead. This is a small price to pay for a font that covers every Unicode
//! character.
//!
//! # Usage
//!
//! Single-threaded usage is simple, via the [`Unifont`](struct.Unifont.html)
//! struct:
//!
//! ```rust
//! use unifont_bitmap::Unifont;
//! let mut unifont = Unifont::open();
//! // Get a bitmap, loading its page if necessary. Requires mut.
//! let my_bitmap = unifont.load_bitmap('井' as u32);
//! println!("{} pixels wide.", if my_bitmap.is_wide() { 16 } else { 8 });
//! println!("Bytes: {:?}", my_bitmap.get_bytes());
//! // Get a bitmap, iff its page is already loaded. Does not require mut.
//! let my_bitmap = unifont.get_bitmap('井' as u32).unwrap();
//! println!("{} pixels wide.", if my_bitmap.is_wide() { 16 } else { 8 });
//! println!("Bytes: {:?}", my_bitmap.get_bytes());
//! ```
//!
//! What you do from here is complicated, and outside this crate's pay grade.
//!
//! # Legalese
//!
//! The `unifont-bitmap` crate is copyright 2021, Solra Bizna, and licensed
//! under either of:
//!
//!  * Apache License, Version 2.0
//!    ([LICENSE-APACHE](LICENSE-APACHE) or
//!    <http://www.apache.org/licenses/LICENSE-2.0>)
//!  * MIT license
//!    ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
//!
//! at your option.
//!
//! As for GNU Unifont:
//!
//! > Copyright (C) 1998-2021 Roman Czyborra, Paul Hardy, Qianqian Fang,
//! > Andrew Miller, Johnnie Weaver, David Corbett, Nils Moskopp, Rebecca
//! > Bettencourt, et al. License: SIL Open Font License version 1.1 and
//! > GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>
//! > with the GNU Font Embedding Exception.
//!
//! I believe that this license is compatible with `unifont-bitmap`'s use of
//! the font. If the font ends up statically linked into a non-GPL-compatible
//! application, e.g. for its own use in UI elements, my interpretation of the
//! license is that this is equivalent to embedding it into a document; thus
//! explicitly permitted by the Font Embedding Exception. If one of the
//! copyright holders and/or the Free Software Foundation disagrees with this
//! interpretation, I'd be open to discuss the issue.
//!
//! ## Contribution
//!
//! Unless you explicitly state otherwise, any contribution intentionally
//! submitted for inclusion in the `unifont-bitmap` crate by you, as defined
//! in the Apache-2.0 license, shall be dual licensed as above, without any
//! additional terms or conditions.

use byteorder::{ReadBytesExt, BigEndian};

const UNIFONT_DATA: &[u8] = include_bytes!("unifont.dat");

/// The largest codepoint value that is, or ever will be, legal in Unicode.
pub const MAX_UNICODE_CODEPOINT: u32 = 0x10FFFF;
/// The number of legal codepoint values that exist in Unicode.
pub const NUM_UNICODE_CODEPOINTS: u32 = MAX_UNICODE_CODEPOINT + 1;
/// The largest number of a 256-codepoint "page" that exists in Unicode.
pub const MAX_UNICODE_PAGE: u32 = NUM_UNICODE_PAGES-1;
/// The number of 256-codepoint "pages" that exist in Unicode.
pub const NUM_UNICODE_PAGES: u32 = NUM_UNICODE_CODEPOINTS >> 8;

/// A single 8x16 or 16x16 bitmap, corresponding to a single displayed glyph.
/// See the module documentation for a cryptic warning about combining
/// characters, invisible characters, etc.
#[derive(PartialEq,Eq)]
pub struct Bitmap<'a> {
    bytes: &'a [u8],
}

impl<'a> core::fmt::Debug for Bitmap<'a> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
	fmt.write_str("unifont bitmap {")?;
	core::fmt::Debug::fmt(self.bytes, fmt)?;
	fmt.write_str("}")
    }
}

impl<'a> Bitmap<'a> {
    /// Returns the bytes that make up the given bitmap. Each byte contains 8
    /// pixels. The highest order bit of the byte is the leftmost pixel, the
    /// next highest order bit is the next pixel, and so on. If the glyph is
    /// wide (see `is_wide`) then there are two bytes per row, otherwise there
    /// is one byte per row.
    pub fn get_bytes(&self) -> &'a [u8] { self.bytes }
    /// Returns `true` if the bitmap is wide (16x16), `false` if it is narrow
    /// (8x16).
    pub fn is_wide(&self) -> bool {
	match self.bytes.len() {
	    16 => false,
	    32 => true,
	    _ => unreachable!(),
	}
    }
    /// Returns the dimensions of the bitmap, width then height.
    /// Always returns (8,16) or (16,16).
    pub fn get_dimensions<T: From<u8>>(&self) -> (T, T) {
	match self.is_wide() {
	    false => (8.into(), 16.into()),
	    true => (16.into(), 16.into()),
	}
    }
}

#[derive(Default)]
struct PageInfo {
    uncompressed_size: u32,
    compressed_offset: u32,
    raw_data: Option<Vec<u8>>,
}

/// A data structure for caching Unifont character bitmaps. Decompresses the
/// compressed font data in the executable on demand, and caches it in blocks
/// ("pages") of 256 code points each.
pub struct Unifont {
    pages: [PageInfo; NUM_UNICODE_PAGES as usize],
}

impl Unifont {
    /// Loads the Unifont bitmap corresponding to the given Unicode codepoint
    /// (if necessary), and returns it.
    ///
    /// Will return the bitmap for U+FFFD REPLACEMENT CHAR (�) if Unifont does
    /// not include a glyph for this bitmap.
    ///
    /// **PANICS** if you pass a `codepoint` larger than
    /// `MAX_UNICODE_CODEPOINT`.
    pub fn load_bitmap(&mut self, codepoint: u32) -> Bitmap {
	assert!(codepoint <= MAX_UNICODE_CODEPOINT);
	let page = codepoint >> 8;
	self.load_page(page);
	let ret = self.get_bitmap(codepoint);
	// Justification for this unsafe block:
	//
	// Once loaded, the decompressed data for a given page will never be
	// freed or moved until (and unless) this Unifont instance is dropped.
	// Therefore, the implied lifetime constraint is met.
	//
	// A previous iteration of this API had a "purge_page" call, but that
	// broke this safety assumption and was therefore removed.
	if let Some(x) = ret { return unsafe { std::mem::transmute(x) } }
	drop(ret);
	if codepoint == 0xFFFD {
	    panic!("U+FFFD should have been loaded but wasn't!");
	}
	else {
	    // this will happen if U+FFFD was needed but not yet loaded
	    self.load_bitmap(0xFFFD)
	}
    }
    /// Gets the Unifont bitmap corresponding to the given Unicode codepoint,
    /// if and only if it is already loaded.
    ///
    /// Will return the bitmap for `U+FFFD REPLACEMENT CHAR` (�) if Unifont
    /// does not include a glyph for this bitmap, iff the respective page of
    /// the font is already loaded.
    ///
    /// **PANICS** if you pass a `codepoint` larger than
    /// `MAX_UNICODE_CODEPOINT`.
    pub fn get_bitmap(&self, codepoint: u32) -> Option<Bitmap> {
	assert!(codepoint <= MAX_UNICODE_CODEPOINT);
	let page = codepoint >> 8;
	let ch = codepoint & 255;
	let raw_data = match self.pages[page as usize].raw_data.as_ref() {
	    None => return None,
	    Some(x) => &x[..],
	};
	let offset_offset = (ch as usize) * 2;
	let char_offset =
	    u16::from_ne_bytes(raw_data[offset_offset .. offset_offset + 2]
			       .try_into().unwrap());
	if char_offset == 0 {
	    if codepoint == 0xFFFD {
		panic!("U+FFFD should have been present but wasn't!");
	    }
	    else {
		self.get_bitmap(0xFFFD)
	    }
	}
	else {
	    let is_wide = (char_offset & 1) != 0;
	    let real_offset = (char_offset & !1) as usize;
	    let region = &raw_data[real_offset .. real_offset +
				   if is_wide { 32 } else { 16 }];
	    Some(Bitmap { bytes: region })
	}
    }
    /// Loads a given page, if it's not loaded already. (Since loading is
    /// usually done transparently, this isn't usually needed.)
    pub fn load_page(&mut self, page: u32) {
	assert!(page <= MAX_UNICODE_PAGE);
	let target_page = &mut self.pages[page as usize];
	if target_page.raw_data.is_none() {
	    if target_page.uncompressed_size == 0 {
		target_page.raw_data = Some(vec![0u8; 512]);
	    }
	    else {
		let mut inflater = flate2::Decompress::new(true);
		let mut buf = vec![0; target_page.uncompressed_size as usize];
		inflater.decompress(&UNIFONT_DATA[target_page.compressed_offset as usize ..], &mut buf[..], flate2::FlushDecompress::Finish).expect("The Unifont bitmap data in this application appears to be corrupted!");
		let mut running_offset = 512u16;
		for n in 0 .. 256 {
		    let i = (n * 2) as usize;
		    let in_offset = u16::from_be_bytes(buf[i..i+2].try_into().unwrap());
		    let out_offset;
		    match in_offset {
			0x0000 => {
			    // narrow char,
			    out_offset = running_offset;
			    running_offset += 16;
			},
			0x0001 => {
			    // wide char
			    out_offset = running_offset | 1;
			    running_offset += 32;
			},
			0x0101 => {
			    // invalid char
			    out_offset = 0;
			},
			_ => {
			    panic!("The Unifont bitmap data in this application appears to be corrupted!");
			},
		    }
		    buf[i..i+2].copy_from_slice(&out_offset.to_ne_bytes());
		}
		target_page.raw_data = Some(buf)
	    }
	}
    }
    /// Creates a new instance of this class, with no glyphs cached yet.
    ///
    /// The font data is embedded in your executable, and does not need to be
    /// provided any other way.
    pub fn open() -> Unifont {
	// oh boy, this pain point hasn't been resolved yet
	let mut pages: [std::mem::MaybeUninit<PageInfo>;
			NUM_UNICODE_PAGES as usize]
	    = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
	for el in &mut pages[..] {
	    unsafe { std::ptr::write(el.as_mut_ptr(), PageInfo {
		compressed_offset: 0, uncompressed_size: 0, raw_data: None
	    }) }
	}
	let mut ret = Unifont { pages: unsafe { std::mem::transmute(pages) } };
	ret.populate_page_infos();
	ret
    }
    fn populate_page_infos(&mut self) {
	let mut input = UNIFONT_DATA;
	let start_offset: u32
	    = input.read_u32::<BigEndian>().unwrap() + 4;
	let mut running_offset = start_offset;
	let mut buf = [0u8; NUM_UNICODE_PAGES as usize * 4];
	let mut fish = flate2::Decompress::new(true);
	fish.decompress(&UNIFONT_DATA[4..(running_offset as usize)],
			&mut buf, flate2::FlushDecompress::Finish).unwrap();
	let mut i = &buf[..];
	for el in &mut self.pages[..] {
	    let uncompressed_size = i.read_u16::<BigEndian>().unwrap();
	    let compressed_size = i.read_u16::<BigEndian>().unwrap();
	    el.uncompressed_size = uncompressed_size as u32;
	    if el.uncompressed_size > 0 {
		el.compressed_offset = running_offset;
		running_offset += compressed_size as u32;
	    }
	    else {
		el.compressed_offset = 0;
	    }
	}
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn bogus_page() {
	let mut unifont = Unifont::open();
	let fffd = unifont.load_bitmap(0xFFFD);
	drop(fffd);
	let bad = unifont.load_bitmap(0x104560);
	drop(bad);
	let fffd = unifont.get_bitmap(0xFFFD);
	let bad = unifont.get_bitmap(0x104560);
	assert_eq!(fffd, bad);
    }
}
