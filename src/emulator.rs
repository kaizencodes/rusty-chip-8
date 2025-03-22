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
type Stack = Vec<u16>;
type Instruction = u16;

pub fn run(rom: String, input_rx: Receiver<Key>, output_tx: Sender<window::DisplayBuffer>) {    
    let file: File = File::open(rom).expect("Rom could not be opened.");
    
    let mut emulator = Emulator::init(file);
    loop {                
        let instruction = emulator.fetch();
        let op_code = (instruction >> 12) & 0xF;
        let vx = ((instruction >> 8) & 0xF) as u8;
        let vy = ((instruction >> 4) & 0xF) as u8;
        let address = instruction & 0xFFF;
        let value = (instruction & 0xFF) as u8;
        let short_value = (instruction & 0xF) as u8;

        match op_code {
            0x0 => { emulator.clear_screen(&output_tx) },
            0x1 => {
                emulator.jump(address);
            },
            0x6 => {
                emulator.set_register(vx, value);
            },
            0x7 => {
                emulator.add_to_register(vx, value);
            },
            0xA => {
                emulator.set_index(address);
            },
            0xD => {
                emulator.display(vx, vy, short_value, &output_tx);
            },
            _ => {}
        }

        sleep(SLEEP_DURATION);
    }
}

struct Emulator {
    memory: Memory,
    pc: usize,
    index_register: u16,
    stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    registers: [u8; 0x10],
    display_buffer: [u32; window::WIDTH * window::HEIGHT],
}

impl Emulator {
    fn init(rom: impl std::io::Read) -> Self {
        let mut memory = [0; MEMORY_SIZE];
        
        load_fonts(&mut memory);
        load_program(&mut memory, rom);

        Self {
            memory: memory, 
            pc: PROGRAM_START, 
            index_register: 0x0, 
            stack: Stack::new(), 
            delay_timer: 0x0, 
            sound_timer: 0x0, 
            registers: [0x0; 0x10],
            display_buffer: [0 as u32; window::WIDTH * window::HEIGHT],
        }
    }

    fn fetch(&mut self) -> Instruction {
        let inst = u16::from_be_bytes([self.memory[self.pc], self.memory[self.pc + 1]]);
        self.pc += 2;
        inst
    }

    fn clear_screen(&mut self, output_tx: &Sender<window::DisplayBuffer>) {
        self.display_buffer = [0 as u32; window::WIDTH * window::HEIGHT];
        output_tx.send(self.display_buffer.clone()).unwrap();
    }
    
    fn jump(&mut self, address: u16) {
        self.pc = address as usize;
    }
    
    fn set_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = value
    }
    
    fn add_to_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] += value
    }
    
    fn set_index(&mut self, address: u16) {
        self.index_register = address;
    }
    
    fn display(&mut self, vx: u8, vy: u8, num_of_rows: u8, output_tx: &Sender<window::DisplayBuffer>) {
        let x = self.registers[vx as usize] & (window::WIDTH - 1) as u8;
        let y = self.registers[vy as usize] & (window::HEIGHT - 1) as u8;
    
        self.registers[0xF] = 0;
        for y_offset in 0..num_of_rows {
            if y + y_offset >= window::HEIGHT as u8 {
                break;
            }
    
            let sprite_row_slice = self.memory[self.index_register as usize + y_offset as usize];
            for x_offset in 0..8 {
                if x + x_offset >= window::WIDTH as u8 {
                    break;
                }
                
                let current_sprite_bit = sprite_row_slice >> (7 - x_offset) & 0x1;
                if current_sprite_bit == 0x0 {
                    continue;
                }
                
                let current_pixel = (y+y_offset) as usize * window::HEIGHT + (x+x_offset) as usize;
    
                if self.display_buffer[current_pixel] == 0xFFFFFF {
                    self.registers[0xF] = 0x1;
                }
                
                self.display_buffer[current_pixel] ^= 0xFFFFFF;
            }
        }
        
        output_tx.send(self.display_buffer.clone()).unwrap();
    }
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

fn load_program(memory: &mut Memory, mut rom: impl std::io::Read) {
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).expect("Failed to read ROM");

    let start = PROGRAM_START;
    let end = PROGRAM_START + buffer.len().min(memory.len() - PROGRAM_START);
    memory[start..end].copy_from_slice(&buffer[..(end - start)]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_load_rom() {
        let rom_data = vec![0xAA, 0xBB, 0xCC];
        let rom = Cursor::new(rom_data);
        let emulator = Emulator::init(rom.clone());

        assert_eq!(emulator.memory[PROGRAM_START], 0xAA);
        assert_eq!(emulator.memory[PROGRAM_START + 1], 0xBB);
        assert_eq!(emulator.memory[PROGRAM_START + 2], 0xCC);
    }

}
