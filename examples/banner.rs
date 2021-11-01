use std::{
    io::{BufRead, stdin},
};
use unifont_bitmap::Unifont;

fn banner_print(unifont: &mut Unifont, ink: char, wat: &str) {
    for c in wat.chars() {
	let bitmap = unifont.load_bitmap(c as u32);
	let pitch = if bitmap.is_wide() { 2 } else { 1 };
	for x in 0..bitmap.get_dimensions().0 {
	    for _ in 0 .. 2 {
		for y in (0..16).rev() {
		    for _ in 0 .. 2 {
			let bi = (x/8) + y*pitch;
			let shift = x%8;
			let b = bitmap.get_bytes()[bi];
			if (128 >> shift) & b == 0 {
			    print!(" ");
			}
			else {
			    print!("{}", ink);
			}
		    }
		}
		println!("");
	    }
	}
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
	eprintln!("At least one argument must be specified.\n\
		   \n\
		   Usage: banner [--blocks] [--] [text to output...]\n\
		   \n\
		   If no text is given as arguments, will print text from \
		   standard input.\n\
		   \n\
		   If you want to just do a banner from standard input, \
		   without using --blocks,\ndo: \"banner --\"\n\
		   \n\
		   --blocks: Use a U+2588 FULL BLOCK as \"ink\" instead of \
		   #. May break some\nterminals.\n\
		   \n\
		   Please note that this example makes no attempt to account \
		   for combining\ncharacters or invisibles!");
	std::process::exit(1);
    }
    let mut args = &args[1..];
    let ink = if args.get(0).map(String::as_str) == Some("--blocks") {
	args = &args[1..];
	'\u{2588}'
    } else { '#' };
    let mut unifont = Unifont::open();
    if args.get(0).map(String::as_str) == Some("--") {
	args = &args[1..];
    }
    if args.len() == 0 {
	// read lines and print those as banner
	let stdin = stdin();
	let mut lines = stdin.lock().lines();
	let mut first = true;
	while let Some(line) = lines.next() {
	    let line = line.unwrap();
	    if !first {
		banner_print(&mut unifont, ink, " ");
	    } else { first = true }
	    banner_print(&mut unifont, ink, &line);
	}
    }
    else {
	// print args as banner, separated by space
	let mut first = true;
	for arg in args {
	    if !first {
		banner_print(&mut unifont, ink, " ");
	    } else { first = true }
	    banner_print(&mut unifont, ink, arg);
	}
    }
}
