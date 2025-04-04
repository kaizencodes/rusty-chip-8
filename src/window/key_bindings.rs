use std::collections::HashMap;

use minifb::Key;

pub fn create_bindings() -> HashMap<Key, u16> {
    HashMap::from([        
        (Key::Key1, 0b1 << 1),
        (Key::Key2, 0b1 << 2),
        (Key::Key3, 0b1 << 3),
        (Key::Key4, 0b1 << 12),
        (Key::Q, 0b1 << 4),
        (Key::W, 0b1 << 5),
        (Key::E, 0b1 << 6),
        (Key::R, 0b1 << 13),
        (Key::A, 0b1 << 7),
        (Key::S, 0b1 << 8),
        (Key::D, 0b1 << 9),
        (Key::F, 0b1 << 14),
        (Key::Z, 0b1 << 10),
        (Key::X, 0b1),
        (Key::C, 0b1 << 11),
        (Key::V, 0b1 << 15),
    ])
}
