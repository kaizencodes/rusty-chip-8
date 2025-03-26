use std::fs::File;
use std::num::Wrapping;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::{self, Duration};
use rand::random;
use std::fmt;

use minifb::Key;
use crate::window;

const MEMORY_SIZE: usize = 4096;
// TODO: move it to a config file
const LOOP_RATE: u64 = 700;
const SLEEP_DURATION: Duration = time::Duration::from_nanos(1_000_000_000 / LOOP_RATE);
const CHIP_48_MODE: bool = true;

// backspace not present in the Chip8 keyboard so we treat that as the empty value.
const EMPTY_KEY: Key = Key::Backspace;

type Memory = [u8; MEMORY_SIZE];
type Stack = Vec<u16>;
type Instruction = u16;

pub fn run(rom: String, input_rx: Receiver<Key>, output_tx: Sender<window::DisplayBuffer>) {    
    let file: File = File::open(rom).expect("Rom could not be opened.");
    
    let mut emulator = Emulator::init(file);

    loop {                
        let instruction = emulator.fetch();
        let op_code = (instruction >> 12) & 0xF;
        let vx = ((instruction >> 8) & 0xF) as usize;
        let vy = ((instruction >> 4) & 0xF) as usize;
        let address = instruction & 0xFFF;
        let value = (instruction & 0xFF) as u8;
        let short_value = (instruction & 0xF) as u8;

        // not sure if this should be initialized here or outside the loop, need to run some programs to test.
        let mut last_key: Key = EMPTY_KEY;
        while let Ok(key) = input_rx.try_recv() {
            last_key = key;
        }

        match op_code {
            0x0 => { 
                match value {
                    0xE0 => emulator.clear_screen(&output_tx) ,
                    0xEE => emulator.return_from_subroutine(),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction)                    
                }
            },
            0x1 => { 
                emulator.jump(address) 
            },
            0x2 => {
                emulator.call_subroutine(address)
            }
            0x3 => {
                emulator.skip_if_vx_eq_value(vx, value);
            }
            0x4 => {
                emulator.skip_if_vx_not_eq_value(vx, value);
            }
            0x5 => {
                emulator.skip_if_vx_eq_vy(vx, vy);
            }
            0x6 => {
                emulator.set_vx_to_value(vx, value);
            },
            0x7 => {
                emulator.add_value(vx, value);
            },
            0x8 => {
                match short_value {
                    0x0 => emulator.set_vx_to_vy(vx, vy),
                    0x1 => emulator.or(vx, vy),
                    0x2 => emulator.and(vx, vy),
                    0x3 => emulator.xor(vx, vy),
                    0x4 => emulator.add_vy(vx, vy),
                    0x5 => emulator.sub(vx, vy),
                    0x6 => emulator.right_shift(vx, vy),
                    0x7 => emulator.sub_reversed(vx, vy),
                    0xE => emulator.left_shift(vx, vy),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction),
                }
            }
            0x9 => {
                emulator.skip_if_vx_not_eq_vy(vx as usize, vy as usize);
            }
            0xA => {
                emulator.set_index(address);
            },
            0xB => {
                emulator.jump_with_offset(vx, address);
            },
            0xC => {
                emulator.random(vx, value);
            },
            0xD => {
                emulator.display(vx, vy, short_value, &output_tx);
            },
            0xE => {
                match value {
                    0x9E => emulator.skip_if_key_pressed(vx, last_key),
                    0xA1 => emulator.skip_if_key_not_pressed(vx, last_key),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction)
                }
            },
            0xF => {
                match value {
                    0x07 => emulator.set_vx_to_delay(vx),
                    0x0A => emulator.get_key(vx, last_key),
                    0x15 => emulator.set_delay_to_vx(vx),
                    0x18 => emulator.set_sound_to_vx(vx),
                    0x1E => emulator.add_to_index(vx),
                    0x29 => emulator.set_index_to_font(vx),
                    0x33 => emulator.binary_coded_decimal_conversion(vx),
                    0x55 => emulator.store_in_memory(vx),
                    0x65 => emulator.load_from_memory(vx),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction)
                }
            }
            _ => {
                eprintln!("Unmatched instruction: {:04X}", instruction)
            }
        }

        if debug {
            println!("Instruction: {:04X}", instruction);
            println!("{}", emulator);
            println!("Press N to continue.");
            // while let input_rx.recv()
            while let Ok(key) = input_rx.recv() {
                if key == Key::N {
                    break;
                }
            }
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

    fn call_subroutine(&mut self, address: u16) {
        self.stack.push(self.pc as u16);
        self.pc = address as usize;
    }

    fn return_from_subroutine(&mut self) {
        self.pc = self.stack.pop().expect("Can't return from top level") as usize;
    }

    fn skip_if_vx_eq_value(&mut self, vx: usize, value: u8) {
        if self.registers[vx] == value {
            self.pc += 2;
        }
    }

    fn skip_if_vx_not_eq_value(&mut self, vx: usize, value: u8) {
        if self.registers[vx] != value {
            self.pc += 2;
        }
    }

    fn skip_if_vx_eq_vy(&mut self, vx: usize, vy: usize) {
        if self.registers[vx] == self.registers[vy] {
            self.pc += 2;
        }
    }

    fn skip_if_vx_not_eq_vy(&mut self, vx: usize, vy: usize) {
        if self.registers[vx] != self.registers[vy] {
            self.pc += 2;
        }
    }

    fn skip_if_key_pressed(&mut self, vx: usize, key: Key) {
        if self.registers[vx] == key as u8 {
            self.pc += 2;
        }
    }

    fn skip_if_key_not_pressed(&mut self, vx: usize, key: Key) {
        if self.registers[vx] != key as u8 {
            self.pc += 2;
        }
    }
    
    fn set_vx_to_value(&mut self, vx: usize, value: u8) {
        self.registers[vx] = value
    }

    fn set_vx_to_vy(&mut self, vx: usize, vy: usize) {
        self.registers[vx] = self.registers[vy]
    }

    fn set_vx_to_delay(&mut self, vx: usize) {
        self.registers[vx] = self.delay_timer;
    }

    fn set_delay_to_vx(&mut self, vx: usize) {
        self.delay_timer = self.registers[vx];
    }

    // TODO: make beeping sound when sound_time > 0;
    fn set_sound_to_vx(&mut self, vx: usize) {
        self.sound_timer = self.registers[vx];
    }
    
    fn add_value(&mut self, vx: usize, value: u8) {
        let new_value = Wrapping(self.registers[vx]) + Wrapping(value);
        self.registers[vx] = new_value.0;
    }

    fn add_vy(&mut self, vx: usize, vy: usize) {
        let (new_value, overflow) = self.registers[vx].overflowing_add(self.registers[vy]);
        self.registers[vx] = new_value;

        if overflow {
            self.registers[0xF] = 0x1;
        } else {
            self.registers[0xF] = 0x0;
        }
    }

    fn or(&mut self, vx: usize, vy: usize) {
        self.registers[vx] |= self.registers[vy]
    }

    fn and(&mut self, vx: usize, vy: usize) {
        self.registers[vx] &= self.registers[vy]
    }

    fn xor(&mut self, vx: usize, vy: usize) {
        self.registers[vx] ^= self.registers[vy]
    }

    fn sub(&mut self, vx: usize, vy: usize) {
        let (new_value, underflow) = self.registers[vx].overflowing_sub(self.registers[vy]);
        self.registers[vx] = new_value;
        
        if underflow {
            self.registers[0xF] = 0x0;
        } else {
            self.registers[0xF] = 0x1;
        }
    }

    fn sub_reversed(&mut self, vx: usize, vy: usize) {
        let (new_value, underflow) = self.registers[vy].overflowing_sub(self.registers[vx]);
        self.registers[vx] = new_value;
        
        if underflow {
            self.registers[0xF] = 0x0;
        } else {
            self.registers[0xF] = 0x1;
        }
    }

    fn right_shift(&mut self, vx: usize, vy: usize) {
        if CHIP_48_MODE {
            self.set_vx_to_vy(vx, vy);
        }
        let right_bit = self.registers[vx] & 0xF;
        self.set_vx_to_value(0xF, right_bit);
        self.registers[vx] >>= 1;
    }

    fn left_shift(&mut self, vx: usize, vy: usize) {
        if CHIP_48_MODE {
            self.set_vx_to_vy(vx, vy);
        }
        let left_bit = self.registers[vx] >> 7;
        self.set_vx_to_value(0xF, left_bit);
        self.registers[vx] <<= 1;
    }

    fn jump_with_offset(&mut self, vx: usize, address: u16) {
        let offset: u8;
        if CHIP_48_MODE {
            offset = self.registers[vx];
        } else {
            offset = self.registers[0x0];
        }
        self.pc = address as usize + offset as usize;
    }

    fn random(&mut self, vx: usize, value: u8) {
        self.registers[vx] = random::<u8>() & value
    }
    
    fn set_index(&mut self, address: u16) {
        self.index_register = address;
    }

    fn set_index_to_font(&mut self, vx: usize) {
        // mask the first 4 bits of vx?
        self.index_register = (FONT_START + self.registers[vx] as usize * FONT_LENGTH) as u16;
    }

    fn add_to_index(&mut self, vx: usize) {
        let (new_value, overflow) = self.index_register.overflowing_add(self.registers[vx] as u16);
        
        // this is a special behaviour for Amiga style interpreter. Spacefight 2091 depends on it.
        if overflow {
            self.registers[0xF] = 0x1;
        }

        self.index_register = new_value;
    }

    fn get_key(&mut self, vx: usize, key: Key) {
        if key == EMPTY_KEY {
            self.pc -= 2;
        } else {
            self.registers[vx] = key as u8;
        }
    }

    fn binary_coded_decimal_conversion(&mut self, vx: usize) {
        let value = self.registers[vx];
        
        let first_digit = value / 100;
        self.memory[self.index_register as usize] = first_digit;
        
        let second_digit = (value / 10) % 10;
        self.memory[self.index_register as usize + 1] = second_digit;
        
        let third_digit = (value % 100) % 10;
        self.memory[self.index_register as usize + 2] = third_digit;
    }

    fn store_in_memory(&mut self, vx: usize) {
        for current_reg in 0..vx + 1 {
            self.memory[self.index_register as usize + current_reg as usize] = self.registers[current_reg];
        }

        if !CHIP_48_MODE {
            self.index_register += vx as u16 + 1;
        }
    }

    fn load_from_memory(&mut self, vx: usize) {
        for current_reg in 0..vx + 1 {
            self.registers[current_reg] = self.memory[self.index_register as usize + current_reg as usize];
        }
        
        if !CHIP_48_MODE {
            self.index_register += vx as u16 + 1;
        }
    }
    
    fn display(&mut self, vx: usize, vy: usize, num_of_rows: u8, output_tx: &Sender<window::DisplayBuffer>) {
        let x = self.registers[vx] & (window::WIDTH - 1) as u8;
        let y = self.registers[vy] & (window::HEIGHT - 1) as u8;
    
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

impl fmt::Display for Emulator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PC: {:#X}, I: {:#X}, Delay Timer: {}, Sound Timer: {}\n",
               self.pc, self.index_register, self.delay_timer, self.sound_timer)?;
        write!(f, "Registers: {:?}\n", self.registers)?;
        write!(f, "Stack: {:?}\n", self.stack)?;

        Ok(())
    }
}

const FONT_START: usize = 0x50;
const FONT_LENGTH: usize = 5;
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
