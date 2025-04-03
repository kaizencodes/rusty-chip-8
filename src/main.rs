use std::sync::{Arc, Mutex};
use std::thread;
use anyhow::Result;
use clap::Parser;
use rusty_chip_8::{emulator, window};

/// A chip-8 emulator
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The path to the program to be loaded
    #[arg(short, long)]
    rom: String,

    /// Run in debug mode where instructions are executed step by step after a N keypress.
    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let key_map = Arc::new(Mutex::new(0u16));
    let display_buffer = Arc::new(Mutex::new([0u32; 2048]));
    let display_buffer_clone = Arc::clone(&display_buffer);
    let key_map_clone = Arc::clone(&key_map);

    // emulator is ran in separate thread so it can work independently from the window.
    thread::spawn(move || { emulator::run(args.rom, display_buffer_clone, key_map_clone, args.debug) });
    
    // window has to run on main thread.
    window::run(display_buffer, key_map);

    Ok(())
}
