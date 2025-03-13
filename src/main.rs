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
}

fn main() -> Result<()> {
    let args = Args::parse();

    let (output_tx, output_rx) = mpsc::channel::<window::DisplayBuffer>(); // Display channel
    let (input_tx, input_rx) = mpsc::channel::<minifb::Key>(); // Keyboard input channel
    
    // emulator is ran in separate thread so it can work independently from the window.
    thread::spawn(|| { emulator::run(args.rom, input_rx, output_tx) });
    
    // window has to run on main thread.
    window::run(input_tx, output_rx);

    Ok(())
}
