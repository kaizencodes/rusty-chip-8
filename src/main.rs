use std::sync::mpsc;
use std::thread;
use anyhow::Result;
use clap::Parser;

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

mod emulator {
    use std::fs::File;
    use std::sync::mpsc::{Receiver, Sender};
    use std::thread::sleep;
    use std::time::{self, Duration};

    use minifb::Key;
    use crate::window;

    const MEMORY_SIZE: usize = 4096;
    // TODO: move it to a config file
    const LOOP_RATE: u64 = 700;
    const SLEEP_DURATION: Duration = time::Duration::from_nanos(1_000_000_000 / LOOP_RATE);
    
    type Memory = [u8; MEMORY_SIZE];
    type Instruction = u16;

    pub fn run(rom: String, input_rx: Receiver<Key>, output_tx: Sender<window::DisplayBuffer>) {
        let mut memory: Memory = [0; MEMORY_SIZE];
        let mut pc: usize = 200;
        let mut index_register: u16;
        let mut stack: Vec<u16>;
        let mut delay_timer: u8;
        let mut sound_timer: u8;
        let mut registers: [u8; 16];
        
        let file: File = File::open(rom).expect("Rom could not be opened.");
        load_fonts(&mut memory);
        load_program(&mut memory, file);

        println!("{:02X?}", memory);

        loop {
            while let Ok(key) = input_rx.try_recv() {
                let mut display_buffer =[0 as u32; window::WIDTH * window::HEIGHT];
                // placeholder to test the setup
                // println!("key {:?} pressed", key as usize);
                display_buffer[key as usize] ^= 0xFFFFFF;
    
                output_tx.send(display_buffer.clone()).unwrap();
            }
            
            // fetch
            // decode
            // execute
    
            sleep(SLEEP_DURATION);
        }
    }
    
    fn fetch(pc: &mut usize, ram: &Memory) -> Instruction {
        let inst = u16::from_be_bytes([ram[*pc], ram[*pc + 1]]);
        *pc += 2;
        inst
    }

    const FONT_START: usize = 0x50;
    const FONT_SET: [u8; 80] = [
        0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
        0x20, 0x60, 0x20, 0x20, 0x70, // 1
        0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
        0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
        0x90, 0x90, 0xF0, 0x10, 0x10, // 4
        0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
        0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
        0xF0, 0x10, 0x20, 0x40, 0x40, // 7
        0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
        0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
        0xF0, 0x90, 0xF0, 0x90, 0x90, // A
        0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
        0xF0, 0x80, 0x80, 0x80, 0xF0, // C
        0xE0, 0x90, 0x90, 0x90, 0xE0, // D
        0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
        0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    ];

    fn load_fonts(memory: &mut Memory) {
        memory[FONT_START..FONT_START + FONT_SET.len()].copy_from_slice(&FONT_SET);
    }

    const PROGRAM_START: usize = 0x200;

    fn load_program(ram: &mut Memory, mut rom: impl std::io::Read) {
        let mut buffer = Vec::new();
        rom.read_to_end(&mut buffer).expect("Failed to read ROM");
    
        let start = PROGRAM_START;
        let end = PROGRAM_START + buffer.len().min(ram.len() - PROGRAM_START);
        ram[start..end].copy_from_slice(&buffer[..(end - start)]);
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Cursor;

        #[test]
        fn test_load_small_rom() {
            let mut memory = [0u8; 4096];
            let rom_data = vec![0xAA, 0xBB, 0xCC];
            let rom = Cursor::new(rom_data);

            load_program(&mut memory, rom);

            assert_eq!(memory[PROGRAM_START], 0xAA);
            assert_eq!(memory[PROGRAM_START + 1], 0xBB);
            assert_eq!(memory[PROGRAM_START + 2], 0xCC);
        }

    }
}

mod window {
    use std::sync::mpsc::{Receiver, Sender};
    use minifb::{Key, Window, WindowOptions};

    pub type DisplayBuffer = [u32; 2048];
    
    pub const WIDTH: usize = 64;
    pub const HEIGHT: usize = 32;
    
    // TODO: move it to a config file
    const REFRESH_RATE: usize = 60;

    pub fn run(input_tx: Sender<Key>, output_rx: Receiver<DisplayBuffer>) {
        let mut window = init();
        let mut buffer: DisplayBuffer = [0 as u32; WIDTH * HEIGHT]; // 64x32 framebuffer
        
        loop {                
            if exit(&window) {
                break
            }

            window.get_keys().iter().for_each(|key| {
                input_tx.send(*key).unwrap();
            });
            
            while let Ok(new_buffer) = output_rx.try_recv() {
                buffer = new_buffer;
            }
            window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
        }
    }

    fn exit(window: &Window) -> bool {
        !window.is_open() || window.is_key_down(Key::Escape)
    }

    fn init() -> Window {
        let mut window =Window::new(
            "Rusty Chip-8",
            WIDTH,
            HEIGHT,
            WindowOptions {
                resize: false,
                scale: minifb::Scale::X16, // Scale up for visibility
                ..WindowOptions::default()
            }).unwrap_or_else(|e| panic!("{}", e));
        window.set_target_fps(REFRESH_RATE);

        return window;
    }

}
