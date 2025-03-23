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

        // TODO: send only released keys.
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