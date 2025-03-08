
fn main() {
    let mut ram: [u8; 4096];
    let mut pc: u8;
    let mut index_register: u16;
    let mut stack: Vec<u16>;
    let mut delay_timer: u8;
    let mut sound_timer: u8;
    let mut registers: [u8; 16];

    let mut window = display::init();
    let mut buffer = [0u32; display::WIDTH * display::HEIGHT]; // 64x32 framebuffer
    buffer.fill(0);
    window.render(&buffer);
}

mod display {
    use minifb::{Key, Window, WindowOptions};
    
    pub const WIDTH: usize = 64;
    pub const HEIGHT: usize = 32;

    pub struct Display {
        window: Window
    }

    impl Display {
        pub fn render(&mut self, buffer: &[u32]) {
            while self.window.is_open() && !self.window.is_key_down(Key::Escape) {
                self.window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
            }
        }
    }

    pub fn init() -> Display {
        let mut res = Display {
            window: Window::new(
            "Rusty Chip-8",
            WIDTH,
            HEIGHT,
            WindowOptions {
                resize: false,
                scale: minifb::Scale::X16, // Scale up for visibility
                ..WindowOptions::default()
            }).unwrap_or_else(|e| panic!("{}", e)),
        };
        res.window.set_target_fps(60);
        return res;
    }

}
