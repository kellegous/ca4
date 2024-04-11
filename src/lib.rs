use byteorder::{BigEndian, ByteOrder};
use cairo::Context;
use chrono::Utc;
use memmap::{Mmap, MmapOptions};
use rand::{distributions::Uniform, prelude::Distribution};
use std::num::ParseIntError;
use std::path::Path;
use std::str::FromStr;
use std::{fs, io};

const THEME_SIZE: usize = 20;

const DARKER: f64 = 0.7;
const BRIGHTER: f64 = 1.0 / DARKER;

#[derive(Clone, Copy, Debug)]
pub struct Color {
    a: u8,
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0xff }
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self {
            r,
            g,
            b,
            a: (a * 255.0) as u8,
        }
    }

    pub fn from_rgba_u32(c: u32) -> Self {
        Self::from_rgba(
            ((c >> 16) & 0xff) as u8,
            ((c >> 8) & 0xff) as u8,
            (c & 0xff) as u8,
            ((c >> 24) & 0xff) as f64 / 255.0,
        )
    }

    pub fn from_rgb_u32(c: u32) -> Self {
        Self::from_rgb(
            ((c >> 16) & 0xff) as u8,
            ((c >> 8) & 0xff) as u8,
            (c & 0xff) as u8,
        )
    }

    pub fn with_alpha(&self, a: f64) -> Self {
        Self::from_rgba(self.r, self.g, self.b, a)
    }

    pub fn set(&self, ctx: &Context) {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        if self.a == 0xff {
            ctx.set_source_rgb(r, g, b);
        } else {
            ctx.set_source_rgba(r, g, b, self.a as f64 / 255.0);
        }
    }

    pub fn luminance(&self) -> f64 {
        let r = self.r as f64 / 256.0;
        let g = self.g as f64 / 256.0;
        let b = self.b as f64 / 256.0;
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn as_f64(&self) -> (f64, f64, f64) {
        (
            self.r as f64 / 255.0,
            self.g as f64 / 255.0,
            self.b as f64 / 255.0,
        )
    }

    pub fn r(&self) -> u8 {
        self.r
    }

    pub fn r_f64(&self) -> f64 {
        self.r as f64 / 255.0
    }

    pub fn g(&self) -> u8 {
        self.g
    }

    pub fn g_f64(&self) -> f64 {
        self.g as f64 / 255.0
    }

    pub fn b(&self) -> u8 {
        self.b
    }

    pub fn b_f64(&self) -> f64 {
        self.b as f64 / 255.0
    }

    pub fn alpha(&self) -> f64 {
        self.a as f64 * 255.0
    }

    pub fn brighter(&self, k: f64) -> Self {
        let (r, g, b) = self.as_f64();
        let k = BRIGHTER.powf(k);
        Self {
            a: self.a,
            r: (r * 255.0 * k) as u8,
            g: (g * 255.0 * k) as u8,
            b: (b * 255.0 * k) as u8,
        }
    }

    pub fn darker(&self, k: f64) -> Self {
        let (r, g, b) = self.as_f64();
        let k = DARKER.powf(k);
        Self {
            a: self.a,
            r: (r * 255.0 * k) as u8,
            g: (g * 255.0 * k) as u8,
            b: (b * 255.0 * k) as u8,
        }
    }

    pub fn white() -> Self {
        Self::from_rgb(0xff, 0xff, 0xff)
    }

    pub fn black() -> Self {
        Self::from_rgb(0x00, 0x00, 0x00)
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

pub struct Themes {
    mem: Mmap,
}

impl Themes {
    pub fn open<P: AsRef<Path>>(src: P) -> io::Result<Self> {
        let f = fs::File::open(src)?;
        Ok(Themes {
            mem: unsafe { MmapOptions::new().map(&f)? },
        })
    }

    pub fn get(&self, idx: usize) -> Vec<Color> {
        let off = idx * THEME_SIZE;
        let mut colors = Vec::with_capacity(5);
        for i in 0..5 {
            let b = off + i * 4;
            colors.push(Color::from_rgb_u32(BigEndian::read_u32(
                &self.mem[b..b + 4],
            )));
        }
        colors
    }

    pub fn pick(&self, rng: &mut dyn rand::RngCore) -> (usize, Vec<Color>) {
        let ix = Uniform::new(0, self.len()).sample(rng);
        (ix, self.get(ix))
    }

    pub fn len(&self) -> usize {
        self.mem.len() / THEME_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Seed {
    v: u64,
}

impl Default for Seed {
    fn default() -> Self {
        Self {
            v: Utc::now().timestamp() as u64,
        }
    }
}

impl FromStr for Seed {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(s, 16).map(Seed::new)
    }
}

impl std::fmt::Display for Seed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}", self.v)
    }
}

impl serde::ser::Serialize for Seed {
    fn serialize<S: serde::ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:08x}", self.v))
    }
}

impl Seed {
    pub fn new(v: u64) -> Self {
        Self { v }
    }

    pub fn from_arg(s: &str) -> Result<Seed, String> {
        Self::from_str(s).map_err(|_| format!("invalid seed: {}", s))
    }

    pub fn value(&self) -> u64 {
        self.v
    }
}
