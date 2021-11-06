use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    ffi::OsString,
    io::{BufRead, Write},
};

use regex::Regex;

// These constants are also defined in our crate, but we can't use our crate
// for them, because our crate needs us to be independently able to compile the
// font from .hex files in the first place.
/// The largest codepoint value that is, or ever will be, legal in Unicode.
pub const MAX_UNICODE_CODEPOINT: u32 = 0x10FFFF;
/// The number of legal codepoint values that exist in Unicode.
pub const NUM_UNICODE_CODEPOINTS: u32 = MAX_UNICODE_CODEPOINT + 1;
/// The largest number of a 256-codepoint "page" that exists in Unicode.
pub const MAX_UNICODE_PAGE: u32 = NUM_UNICODE_PAGES-1;
/// The number of 256-codepoint "pages" that exist in Unicode.
pub const NUM_UNICODE_PAGES: u32 = NUM_UNICODE_CODEPOINTS >> 8;

enum Bitmap {
    Narrow([u8; 16]),
    Wide([u8; 32]),
}

fn main() -> std::io::Result<()> {
    let args: Vec<OsString> = std::env::args_os().collect();
    if args.len() != 2 {
	eprintln!("Usage: cat ~/unifont/font/precompiled/unifont{{,_upper}}-\
		   14.0.01.hex | {} output.dat", args[0].to_string_lossy());
	std::process::exit(1);
    }
    let mut active_pages: HashSet<u32> = HashSet::with_capacity(NUM_UNICODE_PAGES as usize);
    let mut bitmaps: HashMap<u32, Bitmap> = HashMap::with_capacity(MAX_UNICODE_CODEPOINT as usize + 1);
    let hex_line_match = Regex::new("^([0-9A-F]{4,6}):([0-9A-F]{32}{1,2})\r?$")
	.unwrap();
    eprintln!("Reading bitmaps...");
    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    for line in stdin.lines() {
	let line = line?;
	let matched = match hex_line_match.captures(&line) {
	    Some(x) => x,
	    None => {
		eprintln!("Unmatched line: {:?}", line);
		continue;
	    },
	};
	let codepoint = u32::from_str_radix(matched.get(1).unwrap().as_str(), 16).unwrap();
	let bitmap: Vec<u8> = matched.get(2).unwrap().as_str().as_bytes().chunks(2).map(|x| u8::from_str_radix(&std::str::from_utf8(x).unwrap(), 16).unwrap()).collect();
	let bitmap = match bitmap.len() {
	    16 => Bitmap::Narrow(bitmap[..].try_into().unwrap()),
	    32 => Bitmap::Wide(bitmap[..].try_into().unwrap()),
	    _ => unreachable!(),
	};
	bitmaps.insert(codepoint, bitmap);
	let page = codepoint >> 8;
	active_pages.insert(page);
    }
    eprintln!("{bitmaps} bitmaps, taking up {bytes} bytes (uncompressed) in \
	       {pages} pages.",
	      pages = active_pages.len(), bitmaps = bitmaps.len(),
	      bytes = bitmaps.iter().fold(0, |sum, bitmap| {
		  sum + match bitmap.1 {
		      Bitmap::Narrow(_) => 16, Bitmap::Wide(_) => 32
		  }
	      }));
    let mut encoded_pages: Vec<(u32, Vec<u8>)>
	= Vec::with_capacity(active_pages.len());
    let mut sizes_buf = Vec::with_capacity(256 * 2);
    let mut bytes_buf = Vec::with_capacity(256 * 32);
    let mut uncompressed_sizes = [0u16; NUM_UNICODE_PAGES as usize];
    let mut compressed_sizes = [0u16; NUM_UNICODE_PAGES as usize];
    eprintln!("Compressing...");
    for page in active_pages.iter() {
	sizes_buf.clear();
	bytes_buf.clear();
	for codepoint in (page << 8) .. (page << 8) + 256 {
	    // we represent the sizes in this weird form so they're more
	    // compressible. post-loading, the sizes will be overwritten
	    // in-place with offsets.
	    match bitmaps.get(&codepoint) {
		// 0x0101 = invalid char
		None => { sizes_buf.push(0x01); sizes_buf.push(0x01); },
		// 0x0000 = narrow char
		Some(Bitmap::Narrow(bits)) => {
		    sizes_buf.push(0x00); sizes_buf.push(0x00);
		    bytes_buf.extend_from_slice(bits);
		},
		// 0x0001 = wide char
		Some(Bitmap::Wide(bits)) => {
		    sizes_buf.push(0x00); sizes_buf.push(0x01);
		    bytes_buf.extend_from_slice(bits);
		},
	    }
	}
	let uncompressed_length = sizes_buf.len() + bytes_buf.len();
	assert!(uncompressed_length <= 32768);
	let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
	e.write_all(&sizes_buf[..]).unwrap();
	e.write_all(&bytes_buf[..]).unwrap();
	let compressed = e.finish().unwrap();
	assert!(compressed.len() <= 65536);
	uncompressed_sizes[*page as usize] = uncompressed_length as u16;
	compressed_sizes[*page as usize] = compressed.len() as u16;
	encoded_pages.push((*page, compressed));
    }
    let uncompressed_size = uncompressed_sizes.iter().fold(0, |tot, wat| {
	tot + *wat as usize
    });
    let compressed_size = compressed_sizes.iter().fold(0, |tot, wat| {
	tot + *wat as usize
    });
    eprintln!("Uncompressed size: {}", uncompressed_size);
    eprintln!("  Compressed size: {}", compressed_size);
    let ratio = uncompressed_size * 100 / compressed_size;
    eprintln!("Compression ratio: 1 to {}.{:02}", ratio / 100, ratio % 100);
    encoded_pages.sort();
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    for page in 0 .. NUM_UNICODE_PAGES as usize {
	e.write_all(&uncompressed_sizes[page].to_be_bytes()).unwrap();
	e.write_all(&compressed_sizes[page].to_be_bytes()).unwrap();
    }
    let compressed_page_table = e.finish().unwrap();
    let mut output = std::fs::File::create(&args[1]).unwrap();
    output.write_all(&(compressed_page_table.len() as u32).to_be_bytes()).unwrap();
    output.write_all(&compressed_page_table).unwrap();
    for (_, bytes) in encoded_pages.iter() {
	output.write_all(bytes).unwrap();
    }
    Ok(())
}
