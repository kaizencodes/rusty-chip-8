use std::sync::mpsc;
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

    let (output_tx, output_rx) = mpsc::channel::<window::DisplayBuffer>(); // Display channel
    let (input_tx, input_rx) = mpsc::channel::<minifb::Key>(); // Keyboard input channel
    
    // emulator is ran in separate thread so it can work independently from the window.
    thread::spawn(move || { emulator::run(args.rom, input_rx, output_tx, args.debug) });
    
    // window has to run on main thread.
    window::run(input_tx, output_rx);

    Ok(())
}
