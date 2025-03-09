use std::sync::mpsc;
use std::thread::{self, sleep};
use std::time::{self, Duration};

const MEMORY_SIZE: usize = 4096;
// TODO: move it to a config file
const LOOP_RATE: u64 = 700;
const SLEEP_DURATION: Duration = time::Duration::from_nanos(1_000_000_000 / LOOP_RATE);

type Ram = [u8; MEMORY_SIZE];
type Instruction = u16;

fn main() {
    let mut ram: Ram = [0; MEMORY_SIZE];
    let mut pc: u8 = 0;
    let mut index_register: u16;
    let mut stack: Vec<u16>;
    let mut delay_timer: u8;
    let mut sound_timer: u8;
    let mut registers: [u8; 16];

    let (output_tx, output_rx) = mpsc::channel::<window::DisplayBuffer>(); // Framebuffer channel
    let (input_tx, input_rx) = mpsc::channel::<minifb::Key>(); // Keyboard input channel
    
    thread::spawn(move || loop {
        let mut display_buffer =[0 as u32; window::WIDTH * window::HEIGHT];

        while let Ok(key) = input_rx.try_recv() {
            // placeholder to test the setup
            println!("key {:?} pressed", key as usize);
            display_buffer[key as usize] ^= 0xFFFFFF;

            output_tx.send(display_buffer.clone()).unwrap();
        }
        
        // fetch
        // let instruction = fetch(&pc, &ram);
        // decode
        // execute

        sleep(SLEEP_DURATION);
    });
    
    window::run(input_tx, output_rx);
}

// fn fetch(pc: &mut u8, ram: &Ram) -> Instruction {
//     let inst = u16::from_be_bytes([ram[*pc as usize], ram[*pc as usize + 1]]);
//     *pc += 2;
//     inst
// }

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
