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
    let mut pc: usize = PROGRAM_START;
    let mut index_register: u16 = 0x0;
    let mut stack: Vec<u16>;
    let mut delay_timer: u8;
    let mut sound_timer: u8;
    let mut registers: [u8; 16] = [0x0; 16];
    
    let file: File = File::open(rom).expect("Rom could not be opened.");
    load_fonts(&mut memory);
    load_program(&mut memory, file);

    println!("{:02X?}", memory);
    let mut display_buffer =[0 as u32; window::WIDTH * window::HEIGHT];

    loop {                
        let instruction = fetch(&mut pc, &memory);
        let op_code = (instruction >> 12) & 0xF;
        let register_x = ((instruction >> 8) & 0xF) as u8;
        let register_y = ((instruction >> 4) & 0xF) as u8;
        let address = instruction & 0xFFF;
        let value = (instruction & 0xFF) as u8;
        let nibble = (instruction & 0xF) as u8;

        match op_code {
            0x0 => { clear_screen(&mut display_buffer, &output_tx) },
            0x1 => {
                println!("inst: {:04X}, jump, pc before {:02X}", instruction, &pc);
                jump(address, &mut pc);
                println!("pc after {:02X}", &pc);
            },
            0x6 => {
                println!("inst: {:04X}, set registers before {:?}", instruction, &registers);
                set_register(&mut registers, register_x, value);
                println!("registers after {:?}", &registers);
            },
            0x7 => {
                println!("inst: {:04X}, add registers before {:?}", instruction, &registers);
                add_to_register(&mut registers, register_x, value);
                println!("registers after {:?}", &registers);
            },
            0xA => {
                println!("inst: {:04X}, set index before {:?}", instruction, &index_register);
                set_index(&mut index_register, address);
                println!("index after {:?}", &index_register);
            },
            0xD => {
                println!("inst: {:04X}, display", instruction);
                display(&mut registers, register_x, register_y, nibble, &mut display_buffer, &index_register, &memory, &output_tx);
            },
            _ => {}
        }

        sleep(SLEEP_DURATION);
    }
}

fn fetch(pc: &mut usize, memory: &Memory) -> Instruction {
    let inst = u16::from_be_bytes([memory[*pc], memory[*pc + 1]]);
    *pc += 0x02;
    inst
}

fn clear_screen(buffer: &mut [u32; 2048], output_tx: &Sender<window::DisplayBuffer>) {
    *buffer = [0 as u32; window::WIDTH * window::HEIGHT];
    output_tx.send(buffer.clone()).unwrap();
}

fn jump(address: u16, pc: &mut usize) {
    *pc = address as usize;
}

fn set_register(registers: &mut [u8; 16], register: u8, value: u8) {
    registers[register as usize] = value
}

fn add_to_register(registers: &mut [u8; 16], register: u8, value: u8) {
    registers[register as usize] += value
}

fn set_index(index_register: &mut u16, address: u16) {
    *index_register = address;
}

fn display(registers: &mut [u8; 16], vx: u8, vy: u8, size: u8, buffer: &mut [u32; 2048], index_register: &u16, memory: &Memory, output_tx: &Sender<window::DisplayBuffer>) {
    let x = registers[vx as usize] & (window::WIDTH - 1) as u8;
    let y = registers[vy as usize] & (window::HEIGHT - 1) as u8;
    println!("coords {}, {}", x, y);

    registers[0xF] = 0;
    for i in 0..size {
        if y + i >= window::HEIGHT as u8 {
            break;
        }

        let sprite_data = memory[*index_register as usize + i as usize];
        println!("sprite_data {:08b}", sprite_data);
        for xi in 0..8 {
            if x + xi >= window::WIDTH as u8 {
                break;
            }
            
            let current_bit = sprite_data >> (7 - xi) & 0x1;
            println!("current_bit {}", current_bit);
            if current_bit == 0x0 {
                continue;
            }
            
            let current_pixel = (y+i) as usize * window::HEIGHT + (x+xi) as usize;
            println!("current_pixel {}", current_pixel);

            if buffer[current_pixel] == 0xFFFFFF {
                registers[0xF] = 0x1;
            }
            
            buffer[current_pixel] ^= 0xFFFFFF;
        }
    }
    
    output_tx.send(buffer.clone()).unwrap();
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
