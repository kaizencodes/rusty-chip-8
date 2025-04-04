use std::fmt;
use std::num::Wrapping;
use std::sync::{Arc, Mutex};

use rand::random;
use timer::Timer;

use crate::window;

mod fonts;
mod timer;

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
}

impl Chip8 {
    pub fn init(rom: impl std::io::Read) -> Self {
        let mut memory = [0; MEMORY_SIZE];

        load_fonts(&mut memory);
        load_program(&mut memory, rom);

        Self {
            memory,
            pc: PROGRAM_START,
            index_register: 0x0,
            stack: Stack::new(),
            delay_timer: Timer::init(),
            sound_timer: Timer::init(),
            registers: [0x0; 0x10],
        }
    }

    pub fn fetch(&mut self) -> Instruction {
        let inst = u16::from_be_bytes([self.memory[self.pc], self.memory[self.pc + 1]]);
        self.pc += 2;
        inst
    }

    // clear screen.
    pub fn op_00e0(&mut self, display_buffer: &Arc<Mutex<window::DisplayBuffer>>) {
        let mut display_buffer = display_buffer.lock().unwrap();
        *display_buffer = [0u32; window::WIDTH * window::HEIGHT];
    }

    // return from subroutine.
    pub fn op_00ee(&mut self) {
        self.pc = self.stack.pop().expect("Can't return from top level") as usize;
    }

    // jump, sets program counter to the given address.
    pub fn op_1nnn(&mut self, address: u16) {
        self.pc = address as usize;
    }

    // call subroutine.
    pub fn op_2nnn(&mut self, address: u16) {
        self.stack.push(self.pc as u16);
        self.pc = address as usize;
    }

    // skip next instruction if vx register equals given value
    pub fn op_3xnn(&mut self, vx: usize, value: u8) {
        if self.registers[vx] == value {
            self.pc += 2;
        }
    }

    // skip next instruction if vx register not equals given value
    pub fn op_4xnn(&mut self, vx: usize, value: u8) {
        if self.registers[vx] != value {
            self.pc += 2;
        }
    }

    // skip next instruction if vx register equals vy register
    pub fn op_5xy0(&mut self, vx: usize, vy: usize) {
        if self.registers[vx] == self.registers[vy] {
            self.pc += 2;
        }
    }

    pub fn op_6xnn(&mut self, vx: usize, value: u8) {
        self.registers[vx] = value
    }

    // add, with overflow.
    pub fn op_7xnn(&mut self, vx: usize, value: u8) {
        let new_value = Wrapping(self.registers[vx]) + Wrapping(value);
        self.registers[vx] = new_value.0;
    }

    pub fn op_8xy0(&mut self, vx: usize, vy: usize) {
        self.registers[vx] = self.registers[vy]
    }

    // binary or, resets vf based on https://github.com/Timendus/chip8-test-suite?tab=readme-ov-file#quirks-test
    pub fn op_8xy1(&mut self, vx: usize, vy: usize) {
        self.registers[vx] |= self.registers[vy];
        self.registers[0xF] = 0x0;
    }

    // binary and, resets vf based on https://github.com/Timendus/chip8-test-suite?tab=readme-ov-file#quirks-test
    pub fn op_8xy2(&mut self, vx: usize, vy: usize) {
        self.registers[vx] &= self.registers[vy];
        self.registers[0xF] = 0x0;
    }

    // binary xor,  resets vf based on https://github.com/Timendus/chip8-test-suite?tab=readme-ov-file#quirks-test
    pub fn op_8xy3(&mut self, vx: usize, vy: usize) {
        self.registers[vx] ^= self.registers[vy];
        self.registers[0xF] = 0x0;
    }

    // add registers together, with overflow.
    pub fn op_8xy4(&mut self, vx: usize, vy: usize) {
        let (new_value, overflow) = self.registers[vx].overflowing_add(self.registers[vy]);
        self.registers[vx] = new_value;

        if overflow {
            self.registers[0xF] = 0x1;
        } else {
            self.registers[0xF] = 0x0;
        }
    }

    // vx = vx - vy, with overflow.
    pub fn op_8xy5(&mut self, vx: usize, vy: usize) {
        let (new_value, underflow) = self.registers[vx].overflowing_sub(self.registers[vy]);
        self.registers[vx] = new_value;

        if underflow {
            self.registers[0xF] = 0x0;
        } else {
            self.registers[0xF] = 0x1;
        }
    }

    // shift right, put the shifted out bit into vf.
    pub fn op_8xy6(&mut self, vx: usize, vy: usize) {
        let right_bit = self.registers[vy] & 0b1;
        (self.registers[vx], _) = self.registers[vy].overflowing_shr(1);
        self.registers[0xF] = right_bit;
    }

    // vx = vy - vx, with overflow.
    pub fn op_8xy7(&mut self, vx: usize, vy: usize) {
        let (new_value, underflow) = self.registers[vy].overflowing_sub(self.registers[vx]);
        self.registers[vx] = new_value;

        if underflow {
            self.registers[0xF] = 0x0;
        } else {
            self.registers[0xF] = 0x1;
        }
    }

    // shift left, put the shifted out bit into vf.
    pub fn op_8xye(&mut self, vx: usize, vy: usize) {
        let left_bit = (self.registers[vy] >> 7) & 0b1;
        (self.registers[vx], _) = self.registers[vy].overflowing_shl(1);
        self.registers[0xF] = left_bit;
    }

    // skip next instruction if vx register not equals vy register
    pub fn op_9xy0(&mut self, vx: usize, vy: usize) {
        if self.registers[vx] != self.registers[vy] {
            self.pc += 2;
        }
    }

    // Set index
    pub fn op_annn(&mut self, address: u16) {
        self.index_register = address;
    }

    // jump with offset
    pub fn op_bnnn(&mut self, _vx: usize, address: u16) {
        let offset = self.registers[0x0];
        self.pc = address as usize + offset as usize;
    }

    // random
    pub fn op_cxnn(&mut self, vx: usize, value: u8) {
        self.registers[vx] = random::<u8>() & value
    }

    // display
    pub fn op_dxyn(
        &mut self,
        vx: usize,
        vy: usize,
        num_of_rows: u8,
        display_buffer: &Arc<Mutex<window::DisplayBuffer>>,
    ) {
        let x = self.registers[vx] & (window::WIDTH - 1) as u8;
        let y = self.registers[vy] & (window::HEIGHT - 1) as u8;

        let mut display_buffer = display_buffer.lock().unwrap();

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

                let current_sprite_bit = (sprite_row_slice >> (7 - x_offset)) & 0x1;
                if current_sprite_bit == 0x0 {
                    continue;
                }

                let current_pixel =
                    (y + y_offset) as usize * window::WIDTH + (x + x_offset) as usize;

                if display_buffer[current_pixel] == 0xFFFFFF {
                    self.registers[0xF] = 0x1;
                }

                display_buffer[current_pixel] ^= 0xFFFFFF;
            }
        }
    }

    // skip if key is pressed
    pub fn op_ex9e(&mut self, vx: usize, key_map: &Arc<Mutex<u16>>) {
        let flag = key_map.lock().unwrap();
        if (*flag >> self.registers[vx]) & 0b1 == 1 {
            self.pc += 2;
        }
    }

    // skip if key is not pressed
    pub fn op_exa1(&mut self, vx: usize, key_map: &Arc<Mutex<u16>>) {
        let flag = key_map.lock().unwrap();
        if (*flag >> self.registers[vx]) & 0b1 == 0 {
            self.pc += 2;
        }
    }

    // set vx to delay timer
    pub fn op_fx07(&mut self, vx: usize) {
        self.registers[vx] = self.delay_timer.get();
    }

    // block until a key is pressed, set it to vx.
    pub fn op_fx0a(&mut self, vx: usize, key_map: &Arc<Mutex<u16>>) {
        let flag = key_map.lock().unwrap();
        if *flag == 0x00 {
            self.pc -= 2;
        } else {
            // taking the most significant bit as the pressed button.
            let val = flag.ilog(2);
            self.registers[vx] = val as u8;
        }
    }

    // set delay timer
    pub fn op_fx15(&mut self, vx: usize) {
        self.delay_timer.set(self.registers[vx]);
    }

    // set sound timer
    pub fn op_fx18(&mut self, vx: usize) {
        self.sound_timer.set(self.registers[vx]);
    }

    // add to index with overflow
    pub fn op_fx1e(&mut self, vx: usize) {
        let (new_value, overflow) = self
            .index_register
            .overflowing_add(self.registers[vx] as u16);

        // this is a special behaviour for Amiga style interpreter. Spacefight 2091 depends on it.
        if overflow {
            self.registers[0xF] = 0x1;
        }

        self.index_register = new_value;
    }

    // set index to font
    pub fn op_fx29(&mut self, vx: usize) {
        // mask the first 4 bits of vx?
        self.index_register = (fonts::START + self.registers[vx] as usize * fonts::LENGTH) as u16;
    }

    // binary-coded decimal conversion
    pub fn op_fx33(&mut self, vx: usize) {
        let value = self.registers[vx];

        let first_digit = value / 100;
        self.memory[self.index_register as usize] = first_digit;

        let second_digit = (value / 10) % 10;
        self.memory[self.index_register as usize + 1] = second_digit;

        let third_digit = (value % 100) % 10;
        self.memory[self.index_register as usize + 2] = third_digit;
    }

    // save to memory
    pub fn op_fx55(&mut self, vx: usize) {
        for current_reg in 0..vx + 1 {
            self.memory[self.index_register as usize + current_reg] = self.registers[current_reg];
        }

        self.index_register += vx as u16 + 1;
    }

    // load from memory
    pub fn op_fx65(&mut self, vx: usize) {
        for current_reg in 0..vx + 1 {
            self.registers[current_reg] = self.memory[self.index_register as usize + current_reg];
        }

        self.index_register += vx as u16 + 1;
    }
}

impl fmt::Display for Chip8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "PC: {:#X}, I: {:#X}, Delay Timer: {}, Sound Timer: {}",
            self.pc,
            self.index_register,
            self.delay_timer.get(),
            self.sound_timer.get()
        )?;
        writeln!(f, "Registers: {:?}", self.registers)?;
        writeln!(f, "Stack: {:?}", self.stack)?;

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

    #[test]
    fn test_op_00e0() {
        use std::sync::{Arc, Mutex};

        let display_buffer = Arc::new(Mutex::new([0xFFFFFFFF; window::WIDTH * window::HEIGHT]));
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.op_00e0(&display_buffer);

        let buffer = display_buffer.lock().unwrap();
        let expected_result = [0x0; window::WIDTH * window::HEIGHT];

        assert_eq!(*buffer, expected_result);
    }

    #[test]
    fn test_op_00ee() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.stack.push(0x200);

        emulator.op_00ee();

        assert_eq!(emulator.pc, 0x200);
    }

    #[test]
    #[should_panic(expected = "Can't return from top level")]
    fn test_op_00ee_empty_stack() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.op_00ee();
    }

    #[test]
    fn test_op_1nnn() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.op_1nnn(0x300);

        assert_eq!(emulator.pc, 0x300);
    }

    #[test]
    fn test_op_2nnn() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.pc = 0x200;

        emulator.op_2nnn(0x400); // Call subroutine at address 0x400

        assert_eq!(emulator.pc, 0x400); // Ensure PC jumps to new address
        assert_eq!(emulator.stack.last(), Some(&0x200)); // Ensure the previous PC is stored in the stack
    }

    #[test]
    fn test_op_3xnn_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.pc = 0x200;
        emulator.registers[3] = 0x42;

        emulator.op_3xnn(3, 0x42);

        assert_eq!(emulator.pc, 0x202);
    }

    #[test]
    fn test_op_3xnn_no_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.pc = 0x200;
        emulator.registers[3] = 0x41;

        emulator.op_3xnn(3, 0x42);

        assert_eq!(emulator.pc, 0x200);
    }

    #[test]
    fn test_op_4xnn_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.pc = 0x200;
        emulator.registers[3] = 0x41;

        emulator.op_4xnn(3, 0x42);

        assert_eq!(emulator.pc, 0x202);
    }

    #[test]
    fn test_op_4xnn_no_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.pc = 0x200;
        emulator.registers[3] = 0x42;

        emulator.op_4xnn(3, 0x42);

        assert_eq!(emulator.pc, 0x200);
    }

    #[test]
    fn test_op_5xy0_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.pc = 0x200;
        emulator.registers[3] = 0x42;
        emulator.registers[4] = 0x42;

        emulator.op_5xy0(3, 4);

        assert_eq!(emulator.pc, 0x202);
    }

    #[test]
    fn test_op_5xy0_no_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.pc = 0x200;
        emulator.registers[3] = 0x42;
        emulator.registers[4] = 0x41;

        emulator.op_5xy0(3, 4);

        assert_eq!(emulator.pc, 0x200);
    }

    #[test]
    fn test_op_6xnn() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0x00;

        emulator.op_6xnn(3, 0x42);

        assert_eq!(emulator.registers[3], 0x42);
    }

    #[test]
    fn test_op_7xnn() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0x10;
        emulator.op_7xnn(3, 0x20);

        assert_eq!(emulator.registers[3], 0x30);
    }

    #[test]
    fn test_op_7xnn_with_overflow() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0xFF;

        emulator.op_7xnn(3, 0x02);

        assert_eq!(emulator.registers[3], 0x01);
    }

    #[test]
    fn test_op_8xy0() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0x42;
        emulator.registers[4] = 0x99;

        // set vx to vy
        emulator.op_8xy0(3, 4);

        assert_eq!(emulator.registers[3], 0x99);
        assert_eq!(emulator.registers[4], 0x99);
    }

    #[test]
    fn test_op_8xy1() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0b1010;
        emulator.registers[4] = 0b1100;
        emulator.registers[0xF] = 0x1; // setting it to 1 to make sure it's reset to 0.

        // OR
        emulator.op_8xy1(3, 4);

        assert_eq!(emulator.registers[3], 0b1110);
        assert_eq!(emulator.registers[0xF], 0x0);
    }

    #[test]
    fn test_op_8xy2() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0b1010;
        emulator.registers[4] = 0b1100;
        emulator.registers[0xF] = 0x1; // setting it to 1 to make sure it's reset to 0.

        // AND
        emulator.op_8xy2(3, 4);

        assert_eq!(emulator.registers[3], 0b1000);
        assert_eq!(emulator.registers[0xF], 0x0);
    }

    #[test]
    fn test_op_8xy3() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0b1010;
        emulator.registers[4] = 0b1100;
        emulator.registers[0xF] = 0x1; // setting it to 1 to make sure it's reset to 0.

        // XOR
        emulator.op_8xy3(3, 4);

        assert_eq!(emulator.registers[3], 0b0110);
        assert_eq!(emulator.registers[0xF], 0x0);
    }

    #[test]
    fn test_op_8xy4() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0x05;
        emulator.registers[4] = 0x03;

        emulator.op_8xy4(3, 4);

        assert_eq!(emulator.registers[3], 0x08);
        assert_eq!(emulator.registers[0xF], 0x0);
    }

    #[test]
    fn test_op_8xy4_with_overflow() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0xFF;
        emulator.registers[4] = 0x01;

        emulator.op_8xy4(3, 4);

        assert_eq!(emulator.registers[3], 0x00);
        assert_eq!(emulator.registers[0xF], 0x1);
    }

    #[test]
    fn test_op_8xy5() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0x05;
        emulator.registers[4] = 0x03;

        emulator.op_8xy5(3, 4);

        assert_eq!(emulator.registers[3], 0x02);
        assert_eq!(emulator.registers[0xF], 0x1);
    }

    #[test]
    fn test_op_8xy5_with_underflow() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0x03;
        emulator.registers[4] = 0x05;

        emulator.op_8xy5(3, 4);

        assert_eq!(emulator.registers[3], 0xFE);
        assert_eq!(emulator.registers[0xF], 0x0);
    }

    #[test]
    fn test_op_8xy6() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0b0000_0010;

        emulator.op_8xy6(2, 3);

        assert_eq!(emulator.registers[2], 0b0000_0001);
        assert_eq!(emulator.registers[0xF], 0);
    }

    #[test]
    fn test_op_8xy6_with_overflow() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[3] = 0b0000_0011;

        emulator.op_8xy6(2, 3);

        assert_eq!(emulator.registers[2], 0b0000_0001);
        assert_eq!(emulator.registers[0xF], 1);
    }

    #[test]
    fn test_op_8xy7() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[1] = 0x06;
        emulator.registers[2] = 0x0A;

        emulator.op_8xy7(1, 2);

        assert_eq!(emulator.registers[1], 0x04); // 0x0A - 0x06 = 0x04
        assert_eq!(emulator.registers[0xF], 0x1); // no borrow
    }

    #[test]
    fn test_op_8xy7_with_borrow() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[1] = 0x0A;
        emulator.registers[2] = 0x06;

        emulator.op_8xy7(1, 2);

        assert_eq!(emulator.registers[1], 0xFC); // 0x06 - 0x0A = underflow to 0xFC
        assert_eq!(emulator.registers[0xF], 0x0); // borrow
    }

    #[test]
    fn test_op_8xye() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[1] = 0b0010_0001;
        emulator.op_8xye(0, 1);

        assert_eq!(emulator.registers[0], 0b0100_0010);
        assert_eq!(emulator.registers[0xF], 0);
    }

    #[test]
    fn test_op_8xye_with_overflow() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[1] = 0b1000_0001;
        emulator.op_8xye(0, 1);

        assert_eq!(emulator.registers[0], 0b0000_0010);
        assert_eq!(emulator.registers[0xF], 1);
    }

    #[test]
    fn test_op_9xy0_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[2] = 0xAB;
        emulator.registers[3] = 0xCD;
        emulator.pc = 0x200;

        emulator.op_9xy0(2, 3);

        assert_eq!(emulator.pc, 0x202);
    }

    #[test]
    fn test_op_9xy0_no_skip() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[2] = 0x42;
        emulator.registers[3] = 0x42;
        emulator.pc = 0x200;

        emulator.op_9xy0(2, 3);

        assert_eq!(emulator.pc, 0x200);
    }

    #[test]
    fn test_op_annn() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.op_annn(0x456);

        assert_eq!(emulator.index_register, 0x456);
    }

    #[test]
    fn test_op_bnnn() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[0x0] = 0x10;

        emulator.op_bnnn(0, 0x200);
        assert_eq!(emulator.pc, 0x210);
    }

    #[test]
    fn test_op_dxyn() {
        use std::sync::{Arc, Mutex};

        let display_buffer = Arc::new(Mutex::new([0xFFFFFF; window::WIDTH * window::HEIGHT]));
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.registers[0] = 10; // Set vx (x position)
        emulator.registers[1] = 5; // Set vy (y position)
        emulator.index_register = 0;

        // Set a sample sprite (a single row: 0xF0, which is 11110000 in binary)
        emulator.memory[0] = 0xF0;

        let num_of_rows = 1;

        emulator.op_dxyn(0, 1, num_of_rows, &display_buffer);

        let mut expected_result = [0xFFFFFF; window::WIDTH * window::HEIGHT];
        expected_result[5 * window::WIDTH + 10] = 0x0;
        expected_result[5 * window::WIDTH + 11] = 0x0;
        expected_result[5 * window::WIDTH + 12] = 0x0;
        expected_result[5 * window::WIDTH + 13] = 0x0;

        let buffer = display_buffer.lock().unwrap();

        assert_eq!(*buffer, expected_result);
        assert_eq!(emulator.registers[0xF], 0x1);
    }

    #[test]
    fn test_op_ex9e() {
        use std::sync::{Arc, Mutex};

        let key_map = Arc::new(Mutex::new(0xF0u16)); // Example key map: 11110000
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.registers[0] = 0;
        emulator.op_ex9e(0, &key_map);

        // Assert that the program counter is not incremented since key 0 is not pressed
        // 0x200 is the program start location
        assert_eq!(emulator.pc, 0x200);

        emulator.registers[0] = 4;
        emulator.op_ex9e(0, &key_map);

        // Assert that the program counter is incremented because key 4 is pressed
        assert_eq!(emulator.pc, 0x202);
    }

    #[test]
    fn test_op_exa1() {
        use std::sync::{Arc, Mutex};

        let key_map = Arc::new(Mutex::new(0xF0u16)); // Example key map: 11110000
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.registers[0] = 4;
        emulator.op_exa1(0, &key_map);

        // Assert that the program counter is incremented since key 0 is not pressed
        // 0x200 is the program start location
        assert_eq!(emulator.pc, 0x200);

        emulator.registers[0] = 0;
        emulator.op_exa1(0, &key_map);

        // Assert that the program counter is not incremented because key 4 is pressed
        assert_eq!(emulator.pc, 0x202);
    }

    #[test]
    fn test_op_fx0a() {
        use std::sync::{Arc, Mutex};

        // Initialize the key_map (0x10 means key 4 is pressed  0001 0000)
        let key_map = Arc::new(Mutex::new(0x10u16));
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.op_fx0a(0, &key_map);

        assert_eq!(emulator.registers[0], 4);
    }

    #[test]
    fn test_op_fx0a_no_key_press() {
        use std::sync::{Arc, Mutex};
        let key_map = Arc::new(Mutex::new(0x00u16)); // empty keymap
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.op_fx0a(2, &key_map);

        // Assert that the program counter has decreased by 2 (indicating the instruction was skipped)
        assert_eq!(emulator.pc, 0x1FE); // initial 0x200 - 0x2 = 0x1FE
    }

    #[test]
    fn test_op_fx1e() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.index_register = 0x1000;
        emulator.registers[0] = 0x1;

        emulator.op_fx1e(0);

        assert_eq!(emulator.index_register, 0x1001);
        assert_eq!(emulator.registers[0xF], 0x0);
    }

    #[test]
    fn test_op_fx1e_with_overflow() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.index_register = 0xFFFF;
        emulator.registers[0] = 0x1;

        emulator.op_fx1e(0);

        assert_eq!(emulator.index_register, 0x0000);
        assert_eq!(emulator.registers[0xF], 0x1);
    }

    #[test]
    fn test_op_fx29() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[0] = 3;

        emulator.op_fx29(0);

        let expected_index = (fonts::START + 3 * fonts::LENGTH) as u16;
        assert_eq!(emulator.index_register, expected_index);
    }

    #[test]
    fn test_op_fx33() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));
        emulator.registers[0] = 234;

        emulator.op_fx33(0);

        assert_eq!(emulator.memory[emulator.index_register as usize], 2);
        assert_eq!(emulator.memory[emulator.index_register as usize + 1], 3);
        assert_eq!(emulator.memory[emulator.index_register as usize + 2], 4);
    }

    #[test]
    fn test_op_fx55() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.registers[0] = 0x10;
        emulator.registers[1] = 0x20;
        emulator.registers[2] = 0x30;
        emulator.registers[3] = 0x40;
        emulator.registers[4] = 0x50; // should not be saved

        emulator.index_register = 0x200;

        emulator.op_fx55(3);

        assert_eq!(emulator.memory[0x200], 0x10);
        assert_eq!(emulator.memory[0x201], 0x20);
        assert_eq!(emulator.memory[0x202], 0x30);
        assert_eq!(emulator.memory[0x203], 0x40);
        assert_eq!(emulator.memory[0x204], 0x00);

        assert_eq!(emulator.index_register, 0x204);
    }

    #[test]
    fn test_op_fx65() {
        let mut emulator = Chip8::init(Cursor::new(vec![]));

        emulator.index_register = 0x300;
        emulator.memory[0x300] = 0xAA;
        emulator.memory[0x301] = 0xBB;
        emulator.memory[0x302] = 0xCC;
        emulator.memory[0x303] = 0xDD;
        emulator.memory[0x304] = 0xEE; // should not be loaded

        emulator.op_fx65(3);

        assert_eq!(emulator.registers[0], 0xAA);
        assert_eq!(emulator.registers[1], 0xBB);
        assert_eq!(emulator.registers[2], 0xCC);
        assert_eq!(emulator.registers[3], 0xDD);
        assert_eq!(emulator.registers[4], 0x00);

        assert_eq!(emulator.index_register, 0x304);
    }
}
