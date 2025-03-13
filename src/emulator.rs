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
