use std::fs::File;
use std::thread::sleep;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{self, Duration};
use chip8::Chip8;

use crate::window;
use audio_handler::AudioHandler;

mod chip8;
mod audio_handler;

// TODO: move it to a config file
const LOOP_RATE: u64 = 700;
const SLEEP_DURATION: Duration = time::Duration::from_nanos(1_000_000_000 / LOOP_RATE);

pub fn run(rom: String, output_tx: Sender<window::DisplayBuffer>, key_map: Arc<Mutex<u16>>, debug: bool) {    
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
            0x0 => { 
                match value {
                    0xE0 => chip.clear_screen(&output_tx) ,
                    0xEE => chip.return_from_subroutine(),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction)                    
                }
            },
            0x1 => { 
                chip.jump(address) 
            },
            0x2 => {
                chip.call_subroutine(address)
            }
            0x3 => {
                chip.skip_if_vx_eq_value(vx, value);
            }
            0x4 => {
                chip.skip_if_vx_not_eq_value(vx, value);
            }
            0x5 => {
                chip.skip_if_vx_eq_vy(vx, vy);
            }
            0x6 => {
                chip.set_vx_to_value(vx, value);
            },
            0x7 => {
                chip.add_value(vx, value);
            },
            0x8 => {
                match short_value {
                    0x0 => chip.set_vx_to_vy(vx, vy),
                    0x1 => chip.or(vx, vy),
                    0x2 => chip.and(vx, vy),
                    0x3 => chip.xor(vx, vy),
                    0x4 => chip.add_vy(vx, vy),
                    0x5 => chip.sub(vx, vy),
                    0x6 => chip.right_shift(vx, vy),
                    0x7 => chip.sub_reversed(vx, vy),
                    0xE => chip.left_shift(vx, vy),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction),
                }
            }
            0x9 => {
                chip.skip_if_vx_not_eq_vy(vx as usize, vy as usize);
            }
            0xA => {
                chip.set_index(address);
            },
            0xB => {
                chip.jump_with_offset(vx, address);
            },
            0xC => {
                chip.random(vx, value);
            },
            0xD => {
                chip.display(vx, vy, short_value, &output_tx);
            },
            0xE => {
                match value {
                    0x9E => chip.skip_if_key_pressed(vx, &key_map),
                    0xA1 => chip.skip_if_key_not_pressed(vx, &key_map),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction)
                }
            },
            0xF => {
                match value {
                    0x07 => chip.set_vx_to_delay(vx),
                    0x0A => chip.get_key(vx, &key_map),
                    0x15 => chip.set_delay_to_vx(vx),
                    0x18 => chip.set_sound_to_vx(vx),
                    0x1E => chip.add_to_index(vx),
                    0x29 => chip.set_index_to_font(vx),
                    0x33 => chip.binary_coded_decimal_conversion(vx),
                    0x55 => chip.store_in_memory(vx),
                    0x65 => chip.load_from_memory(vx),
                    _ => eprintln!("Unmatched instruction: {:04X}", instruction)
                }
            }
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
                    break
                }
                drop(flag);
                sleep(SLEEP_DURATION * 10);
            }
        }

        sleep(SLEEP_DURATION);
    }
}
