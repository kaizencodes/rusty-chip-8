use std::sync::{Arc, Mutex};
use key_bindings::create_bindings;
use minifb::{Key, Window, WindowOptions};

pub type DisplayBuffer = [u32; 2048];
pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

// TODO: move it to a config file
const REFRESH_RATE: usize = 60;

mod key_bindings;

pub fn run(display_buffer: Arc<Mutex<DisplayBuffer>>, key_map: Arc<Mutex<u16>>) {
    let mut window = init();
    let mut buffer: DisplayBuffer; // 64x32 framebuffer
    let key_bindings = create_bindings();

    loop {                
        if exit(&window) {
            break
        }

        let mut key_map = key_map.lock().unwrap();
        *key_map = 0x00;
        
        window.get_keys().iter().for_each(|key| {
            if key_bindings.contains_key(key) {
                *key_map ^= key_bindings[key];
            }
        });
        drop(key_map);

        let display_buffer = display_buffer.lock().unwrap();
        buffer = display_buffer.clone();
        drop(display_buffer);

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