//! FPS counter helper.

pub struct FpsCounter {
    frames: u32,
    last_sec: std::time::Instant,
    pub fps: u32,
}

impl FpsCounter {
    pub fn new() -> Self { Self { frames: 0, last_sec: std::time::Instant::now(), fps: 0 } }

    pub fn tick(&mut self) {
        self.frames += 1;
        if self.last_sec.elapsed().as_secs_f32() >= 1.0 {
            self.fps = self.frames;
            self.frames = 0;
            self.last_sec = std::time::Instant::now();
        }
    }
}

// -------- Tiny bitmap text for on-screen FPS overlay ---------

// 5x7 bitmap font for a small subset: digits and 'F','P','S', and ':'
// Each glyph is 7 rows of 5 bits (MSB left), packed into u8 (lower 5 bits used per row)
fn glyph_bits(ch: char) -> Option<[u8; 7]> {
    match ch {
        '0' => Some([0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110]),
        '1' => Some([0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110]),
        '2' => Some([0b01110,0b10001,0b00001,0b00010,0b00100,0b01000,0b11111]),
        '3' => Some([0b11110,0b00001,0b00001,0b01110,0b00001,0b00001,0b11110]),
        '4' => Some([0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010]),
        '5' => Some([0b11111,0b10000,0b11110,0b00001,0b00001,0b10001,0b01110]),
        '6' => Some([0b00110,0b01000,0b10000,0b11110,0b10001,0b10001,0b01110]),
        '7' => Some([0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000]),
        '8' => Some([0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110]),
        '9' => Some([0b01110,0b10001,0b10001,0b01111,0b00001,0b00010,0b01100]),
        'F' => Some([0b11111,0b10000,0b11110,0b10000,0b10000,0b10000,0b10000]),
        'P' => Some([0b11110,0b10001,0b10001,0b11110,0b10000,0b10000,0b10000]),
        'S' => Some([0b01111,0b10000,0b10000,0b01110,0b00001,0b00001,0b11110]),
        ':' => Some([0b00000,0b00100,0b00100,0b00000,0b00100,0b00100,0b00000]),
        ' ' => Some([0b00000,0b00000,0b00000,0b00000,0b00000,0b00000,0b00000]),
        _ => None,
    }
}

pub fn draw_text(buf: &mut [u32], w: usize, x: usize, y: usize, text: &str, fg: u32, bg: Option<u32>) {
    let mut pen_x = x as isize;
    let y = y as isize;
    for ch in text.chars() {
        if let Some(bits) = glyph_bits(ch) {
            // optional background block
            if let Some(bgcol) = bg {
                for ry in 0..7 {
                    let py = y + ry;
                    if py < 0 || (py as usize) >= buf.len() / w { continue; }
                    for rx in 0..6 { // 1px spacing
                        let px = pen_x + rx;
                        if px < 0 || (px as usize) >= w { continue; }
                        buf[py as usize * w + px as usize] = bgcol;
                    }
                }
            }
            // draw glyph 5x7
            for (ry, row) in bits.iter().enumerate() {
                let py = y + ry as isize;
                if py < 0 || (py as usize) >= buf.len() / w { continue; }
                for rx in 0..5 {
                    if (row >> (4 - rx)) & 1 == 1 {
                        let px = pen_x + rx as isize;
                        if px >= 0 && (px as usize) < w {
                            buf[py as usize * w + px as usize] = fg;
                        }
                    }
                }
            }
            pen_x += 6; // 5px glyph + 1px space
        } else {
            pen_x += 4; // fallback spacing
        }
    }
}
