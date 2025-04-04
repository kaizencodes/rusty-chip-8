use chip8::Chip8;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{self, Duration};

use crate::window;
use audio_handler::AudioHandler;

mod audio_handler;
mod chip8;

// TODO: move it to a config file
const LOOP_RATE: u64 = 700;
const SLEEP_DURATION: Duration = time::Duration::from_nanos(1_000_000_000 / LOOP_RATE);

pub fn run(
    rom: String,
    display_buffer: Arc<Mutex<window::DisplayBuffer>>,
    key_map: Arc<Mutex<u16>>,
    debug: bool,
) {
    let file: File = File::open(rom).expect("Rom could not be opened.");

    let mut chip = Chip8::init(file);
    let audio_handler = AudioHandler::init();

    loop {
        audio_handler.tick(chip.sound_timer.get());

        let instruction = chip.fetch();

        let op_code = (instruction >> 12) & 0xF;
        let vx = ((instruction >> 8) & 0xF) as usize;
        let vy = ((instruction >> 4) & 0xF) as usize;
        let address = instruction & 0xFFF;
        let value = (instruction & 0xFF) as u8;
        let short_value = (instruction & 0xF) as u8;

        match op_code {
            0x0 => match value {
                0xE0 => chip.op_00e0(&display_buffer),
                0xEE => chip.op_00ee(),
                _ => eprintln!("Unmatched instruction: {:04X}", instruction),
            },
            0x1 => chip.op_1nnn(address),
            0x2 => chip.op_2nnn(address),
            0x3 => {
                chip.op_3xnn(vx, value);
            }
            0x4 => {
                chip.op_4xnn(vx, value);
            }
            0x5 => {
                chip.op_5xy0(vx, vy);
            }
            0x6 => {
                chip.op_6xnn(vx, value);
            }
            0x7 => {
                chip.op_7xnn(vx, value);
            }
            0x8 => match short_value {
                0x0 => chip.op_8xy0(vx, vy),
                0x1 => chip.op_8xy1(vx, vy),
                0x2 => chip.op_8xy2(vx, vy),
                0x3 => chip.op_8xy3(vx, vy),
                0x4 => chip.op_8xy4(vx, vy),
                0x5 => chip.op_8xy5(vx, vy),
                0x6 => chip.op_8xy6(vx, vy),
                0x7 => chip.op_8xy7(vx, vy),
                0xE => chip.op_8xye(vx, vy),
                _ => eprintln!("Unmatched instruction: {:04X}", instruction),
            },
            0x9 => {
                chip.op_9xy0(vx, vy);
            }
            0xA => {
                chip.op_annn(address);
            }
            0xB => {
                chip.op_bnnn(vx, address);
            }
            0xC => {
                chip.op_cxnn(vx, value);
            }
            0xD => {
                chip.op_dxyn(vx, vy, short_value, &display_buffer);
            }
            0xE => match value {
                0x9E => chip.op_ex9e(vx, &key_map),
                0xA1 => chip.op_exa1(vx, &key_map),
                _ => eprintln!("Unmatched instruction: {:04X}", instruction),
            },
            0xF => match value {
                0x07 => chip.op_fx07(vx),
                0x0A => chip.op_fx0a(vx, &key_map),
                0x15 => chip.op_fx15(vx),
                0x18 => chip.op_fx18(vx),
                0x1E => chip.op_fx1e(vx),
                0x29 => chip.op_fx29(vx),
                0x33 => chip.op_fx33(vx),
                0x55 => chip.op_fx55(vx),
                0x65 => chip.op_fx65(vx),
                _ => eprintln!("Unmatched instruction: {:04X}", instruction),
            },
            _ => {
                eprintln!("Unmatched instruction: {:04X}", instruction)
            }
        }

        if debug {
            println!("Instruction: {:04X}", instruction);
            println!("{}", chip);
            println!("Press C to continue.");
            loop {
                let flag = key_map.lock().unwrap();
                if (*flag >> 11) & 0b1 == 1 {
                    break;
                }
                drop(flag);
                sleep(SLEEP_DURATION * 10);
            }
        }

        sleep(SLEEP_DURATION);
    }
}
