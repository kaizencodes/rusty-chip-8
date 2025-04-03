use std::fmt;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::num::Wrapping;

use rand::random;
use timer::Timer;

use crate::window;

mod timer;
mod fonts;

const MEMORY_SIZE: usize = 4096;
pub(crate) type Memory = [u8; MEMORY_SIZE];
type Stack = Vec<u16>;
type Instruction = u16;

pub struct Chip8 {
    pub memory: Memory,
    pub pc: usize,
    pub index_register: u16,
    pub stack: Vec<u16>,
    pub delay_timer: Timer,
    pub sound_timer: Timer,
    pub registers: [u8; 0x10],
    pub display_buffer: [u32; window::WIDTH * window::HEIGHT],
}

impl Chip8 {
    pub fn init(rom: impl std::io::Read) -> Self {
        let mut memory = [0; MEMORY_SIZE];
        
        load_fonts(&mut memory);
        load_program(&mut memory, rom);

        Self {
            memory: memory, 
            pc: PROGRAM_START, 
            index_register: 0x0, 
            stack: Stack::new(), 
            delay_timer: Timer::init(), 
            sound_timer: Timer::init(), 
            registers: [0x0; 0x10],
            display_buffer: [0 as u32; window::WIDTH * window::HEIGHT],
        }
    }

    pub fn fetch(&mut self) -> Instruction {
        let inst = u16::from_be_bytes([self.memory[self.pc], self.memory[self.pc + 1]]);
        self.pc += 2;
        inst
    }

    pub fn clear_screen(&mut self, output_tx: &Sender<window::DisplayBuffer>) {
        self.display_buffer = [0 as u32; window::WIDTH * window::HEIGHT];
        output_tx.send(self.display_buffer.clone()).unwrap();
    }
    
    pub fn jump(&mut self, address: u16) {
        self.pc = address as usize;
    }

    pub fn call_subroutine(&mut self, address: u16) {
        self.stack.push(self.pc as u16);
        self.pc = address as usize;
    }

    pub fn return_from_subroutine(&mut self) {
        self.pc = self.stack.pop().expect("Can't return from top level") as usize;
    }

    pub fn skip_if_vx_eq_value(&mut self, vx: usize, value: u8) {
        if self.registers[vx] == value {
            self.pc += 2;
        }
    }

    pub fn skip_if_vx_not_eq_value(&mut self, vx: usize, value: u8) {
        if self.registers[vx] != value {
            self.pc += 2;
        }
    }

    pub fn skip_if_vx_eq_vy(&mut self, vx: usize, vy: usize) {
        if self.registers[vx] == self.registers[vy] {
            self.pc += 2;
        }
    }

    pub fn skip_if_vx_not_eq_vy(&mut self, vx: usize, vy: usize) {
        if self.registers[vx] != self.registers[vy] {
            self.pc += 2;
        }
    }

    pub fn skip_if_key_pressed(&mut self, vx: usize, key_map: &Arc<Mutex<u16>>) {
        let flag = key_map.lock().unwrap();
        if (*flag >> self.registers[vx]) & 0b1 == 1 {
            self.pc += 2;
        }
    }

    pub fn skip_if_key_not_pressed(&mut self, vx: usize, key_map: &Arc<Mutex<u16>>) {
        let flag = key_map.lock().unwrap();
        if (*flag >> self.registers[vx]) & 0b1 == 0 {
            self.pc += 2;
        }
    }
    
    pub fn set_vx_to_value(&mut self, vx: usize, value: u8) {
        self.registers[vx] = value
    }

    pub fn set_vx_to_vy(&mut self, vx: usize, vy: usize) {
        self.registers[vx] = self.registers[vy]
    }

    pub fn set_vx_to_delay(&mut self, vx: usize) {
        self.registers[vx] = self.delay_timer.get();
    }

    pub fn set_delay_to_vx(&mut self, vx: usize) {
        self.delay_timer.set(self.registers[vx]);
    }

    // TODO: make beeping sound when sound_time > 0;
    pub fn set_sound_to_vx(&mut self, vx: usize) {
        self.sound_timer.set(self.registers[vx]);
    }
    
    pub fn add_value(&mut self, vx: usize, value: u8) {
        let new_value = Wrapping(self.registers[vx]) + Wrapping(value);
        self.registers[vx] = new_value.0;
    }

    pub fn add_vy(&mut self, vx: usize, vy: usize) {
        let (new_value, overflow) = self.registers[vx].overflowing_add(self.registers[vy]);
        self.registers[vx] = new_value;

        if overflow {
            self.registers[0xF] = 0x1;
        } else {
            self.registers[0xF] = 0x0;
        }
    }

    pub fn or(&mut self, vx: usize, vy: usize) {
        self.registers[vx] |= self.registers[vy];
        self.registers[0xF] = 0x0;
    }

    pub fn and(&mut self, vx: usize, vy: usize) {
        self.registers[vx] &= self.registers[vy];
        self.registers[0xF] = 0x0;
    }

    pub fn xor(&mut self, vx: usize, vy: usize) {
        self.registers[vx] ^= self.registers[vy];
        self.registers[0xF] = 0x0;
    }

    pub fn sub(&mut self, vx: usize, vy: usize) {
        let (new_value, underflow) = self.registers[vx].overflowing_sub(self.registers[vy]);
        self.registers[vx] = new_value;
        
        if underflow {
            self.registers[0xF] = 0x0;
        } else {
            self.registers[0xF] = 0x1;
        }
    }

    pub fn sub_reversed(&mut self, vx: usize, vy: usize) {
        let (new_value, underflow) = self.registers[vy].overflowing_sub(self.registers[vx]);
        self.registers[vx] = new_value;
        
        if underflow {
            self.registers[0xF] = 0x0;
        } else {
            self.registers[0xF] = 0x1;
        }
    }

    pub fn right_shift(&mut self, vx: usize, vy: usize) {
        let right_bit = self.registers[vy] & 0b1;
        (self.registers[vx], _) = self.registers[vy].overflowing_shr(1);
        self.registers[0xF] = right_bit;
    }

    pub fn left_shift(&mut self, vx: usize, vy: usize) {
        let left_bit = (self.registers[vy] >> 7) & 0b1;
        (self.registers[vx], _) = self.registers[vy].overflowing_shl(1);
        self.registers[0xF] = left_bit;
    }

    pub fn jump_with_offset(&mut self, _vx: usize, address: u16) {
        let offset: u8;
        offset = self.registers[0x0];
        
        self.pc = address as usize + offset as usize;
    }

    pub fn random(&mut self, vx: usize, value: u8) {
        self.registers[vx] = random::<u8>() & value
    }
    
    pub fn set_index(&mut self, address: u16) {
        self.index_register = address;
    }

    pub fn set_index_to_font(&mut self, vx: usize) {
        // mask the first 4 bits of vx?
        self.index_register = (fonts::START + self.registers[vx] as usize * fonts::LENGTH) as u16;
    }

    pub fn add_to_index(&mut self, vx: usize) {
        let (new_value, overflow) = self.index_register.overflowing_add(self.registers[vx] as u16);
        
        // this is a special behaviour for Amiga style interpreter. Spacefight 2091 depends on it.
        if overflow {
            self.registers[0xF] = 0x1;
        }

        self.index_register = new_value;
    }

    pub fn get_key(&mut self, vx: usize, key_map: &Arc<Mutex<u16>>) {
        let flag = key_map.lock().unwrap();
        if *flag == 0x00 {
            self.pc -= 2;
        } else {
            // taking the most significant bit as the pressed button.
            let val = flag.ilog(2);
            self.registers[vx] = val as u8;
        }
    }

    pub fn binary_coded_decimal_conversion(&mut self, vx: usize) {
        let value = self.registers[vx];
        
        let first_digit = value / 100;
        self.memory[self.index_register as usize] = first_digit;
        
        let second_digit = (value / 10) % 10;
        self.memory[self.index_register as usize + 1] = second_digit;
        
        let third_digit = (value % 100) % 10;
        self.memory[self.index_register as usize + 2] = third_digit;
    }

    pub fn store_in_memory(&mut self, vx: usize) {
        for current_reg in 0..vx + 1 {
            self.memory[self.index_register as usize + current_reg as usize] = self.registers[current_reg];
        }

        self.index_register += vx as u16 + 1;
    }

    pub fn load_from_memory(&mut self, vx: usize) {
        for current_reg in 0..vx + 1 {
            self.registers[current_reg] = self.memory[self.index_register as usize + current_reg as usize];
        }
        
        self.index_register += vx as u16 + 1;
    }
    
    pub fn display(&mut self, vx: usize, vy: usize, num_of_rows: u8, output_tx: &Sender<window::DisplayBuffer>) {
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
                
                let current_pixel = (y+y_offset) as usize * window::WIDTH + (x+x_offset) as usize;
    
                if self.display_buffer[current_pixel] == 0xFFFFFF {
                    self.registers[0xF] = 0x1;
                }
                
                self.display_buffer[current_pixel] ^= 0xFFFFFF;
            }
        }
        
        output_tx.send(self.display_buffer.clone()).unwrap();
    }
}

impl fmt::Display for Chip8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PC: {:#X}, I: {:#X}, Delay Timer: {}, Sound Timer: {}\n",
                self.pc, self.index_register, self.delay_timer.get(), self.sound_timer.get())?;
        write!(f, "Registers: {:?}\n", self.registers)?;
        write!(f, "Stack: {:?}\n", self.stack)?;

        Ok(())
    }
}

const PROGRAM_START: usize = 0x200;

fn load_program(memory: &mut Memory, mut rom: impl std::io::Read) {
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).expect("Failed to read ROM");

    let start = PROGRAM_START;
    let end = PROGRAM_START + buffer.len().min(memory.len() - PROGRAM_START);
    memory[start..end].copy_from_slice(&buffer[..(end - start)]);
}

fn load_fonts(memory: &mut Memory) {
    memory[fonts::START..fonts::START + fonts::FONT_SET.len()].copy_from_slice(&fonts::FONT_SET);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_load_rom() {
        let rom_data = vec![0xAA, 0xBB, 0xCC];
        let rom = Cursor::new(rom_data);
        let emulator = Chip8::init(rom.clone());

        assert_eq!(emulator.memory[PROGRAM_START], 0xAA);
        assert_eq!(emulator.memory[PROGRAM_START + 1], 0xBB);
        assert_eq!(emulator.memory[PROGRAM_START + 2], 0xCC);
    }
}

