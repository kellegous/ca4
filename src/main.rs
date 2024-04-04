use ca1::{Seed, Themes};
use cairo::{Format, ImageSurface};
use clap::Parser;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use std::str::FromStr;
use std::{error::Error, fmt::Debug, fs};

#[derive(Parser, Debug)]
struct Options {
    #[clap(long, default_value_t = 1000)]
    rows: i32,

    #[clap(long, default_value_t = 100)]
    cols: i32,

    #[clap(long, default_value_t = 6)]
    cell_size: i32,

    #[clap(long, value_parser = Rule::from_arg)]
    rule: Option<Rule>,

    #[clap(long, default_value = "themes.bin")]
    themes: String,

    #[clap(long, default_value_t = Default::default(), value_parser = Seed::from_arg)]
    seed: Seed,

    #[clap(long, default_value = "out.png")]
    dest: String,
}

#[derive(Debug, Copy, Clone)]
struct Rule {
    rule: u64,
}

impl Rule {
    fn new(rule: u64) -> Self {
        Self { rule }
    }

    fn apply(&self, p: u8) -> u8 {
        ((self.rule >> p) & 3) as u8
    }

    fn from_arg(s: &str) -> Result<Self, String> {
        Self::from_str(s).map_err(|e| e.to_string())
    }
}

impl FromStr for Rule {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rule = u64::from_str_radix(s, 16)?;
        Ok(Self::new(rule))
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:x}", self.rule)
    }
}

struct State {
    v: Vec<u8>,
}

impl State {
    fn with_size(n: usize) -> Self {
        Self { v: vec![0; n] }
    }

    fn get(&self, i: i32) -> u8 {
        self.v[i.rem_euclid(self.v.len() as i32) as usize]
    }

    fn len(&self) -> usize {
        self.v.len()
    }

    fn set(&mut self, i: i32, value: u8) {
        let n = self.len() as i32;
        self.v[i.rem_euclid(n) as usize] = value;
    }

    fn apply(&self, rule: Rule) -> Self {
        let mut next = Vec::with_capacity(self.v.len());
        for i in 0..self.v.len() {
            let a = self.get(i as i32 - 1);
            let b = self.get(i as i32);
            let c = self.get(i as i32 + 1);
            next.push(rule.apply(a << 4 | b << 2 | c));
        }
        Self { v: next }
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = self
            .v
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "[{}]", s)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse();

    let mut rng = Pcg64::seed_from_u64(options.seed.value());

    let themes = Themes::open(options.themes)?;

    let (theme, colors) = themes.pick(&mut rng);

    let rule = options.rule.unwrap_or(Rule::new(rng.gen()));

    println!("seed: {}, theme: {}, rule: {}", options.seed, theme, rule);

    let width = options.cols * options.cell_size + options.cols + 1;
    let height = options.rows * options.cell_size + options.rows + 1;

    let img = ImageSurface::create(Format::ARgb32, width, height)?;
    let ctx = cairo::Context::new(&img)?;

    colors[4].set(&ctx);
    ctx.rectangle(0.0, 0.0, width as f64, height as f64);
    ctx.fill()?;

    let mut state = State::with_size(options.cols as usize);
    state.set(options.cols / 2, 3);

    for j in 0..options.rows {
        let y = j * (options.cell_size + 1) + 1;
        for i in 0..options.cols {
            let v = state.get(i);
            if v == 0 {
                continue;
            }
            colors[v as usize].set(&ctx);
            let x = i * (options.cell_size + 1) + 1;
            ctx.rectangle(
                x as f64,
                y as f64,
                options.cell_size as f64,
                options.cell_size as f64,
            );
            ctx.fill()?;
        }
        state = state.apply(rule);
    }

    img.write_to_png(&mut fs::File::create(&options.dest)?)?;

    Ok(())
}
