use std::{collections::HashMap, sync::{mpsc::{Receiver, Sender}, Arc, Mutex}};
use minifb::{Key, Window, WindowOptions};

pub type DisplayBuffer = [u32; 2048];

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

// TODO: move it to a config file
const REFRESH_RATE: usize = 60;

pub fn run(output_rx: Receiver<DisplayBuffer>, data: Arc<Mutex<u16>>) {
    let mut window = init();
    let mut buffer: DisplayBuffer = [0 as u32; WIDTH * HEIGHT]; // 64x32 framebuffer
    let key_map = create_keymap();

    loop {                
        if exit(&window) {
            break
        }

        let mut num = data.lock().unwrap();
        *num = 0x00;
        
        window.get_keys().iter().for_each(|key| {
            *num ^= key_map[key];
        });
        drop(num);
        
        while let Ok(new_buffer) = output_rx.try_recv() {
            buffer = new_buffer;
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}

fn create_keymap() -> HashMap<Key, u16> {
    HashMap::from([        
        (Key::Key1, 0b10),
        (Key::Key2, 0b100),
        (Key::Key3, 0b1000),
        (Key::Key4, 0b1000000000000),
        (Key::Q, 0b10000),
        (Key::W, 0b100000),
        (Key::E, 0b1000000),
        (Key::R, 0b10000000000000),
        (Key::A, 0b10000000),
        (Key::S, 0b100000000),
        (Key::D, 0b1000000000),
        (Key::F, 0b100000000000000),
        (Key::Z, 0b10000000000),
        (Key::X, 0b1),
        (Key::C, 0b100000000000),
        (Key::V, 0b1000000000000000),
    ])
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