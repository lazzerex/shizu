use std::env;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

pub const IDLE_DELAY: Duration = Duration::from_secs(5);
pub const FRAME_INTERVAL: Duration = Duration::from_millis(66);
pub const MIN_WIDTH: u16 = 20;
pub const MIN_HEIGHT: u16 = 5;

const SUBCELL: i32 = 16;
const UNICODE_GLYPHS: [char; 3] = ['❀', '✿', '·'];
const ASCII_GLYPHS: [char; 3] = ['.', '*', '\''];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Off,
    Welcome,
    #[default]
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyphs {
    Unicode,
    Ascii,
}

impl Glyphs {
    pub fn detect() -> Self {
        let locale = env::var("LC_ALL").ok().or_else(|| env::var("LANG").ok());
        Self::from_locale(locale.as_deref())
    }

    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(value)
                if value.to_ascii_lowercase().contains("utf-8")
                    || value.to_ascii_lowercase().contains("utf8") =>
            {
                Self::Unicode
            }
            _ => Self::Ascii,
        }
    }

    fn characters(self) -> &'static [char; 3] {
        match self {
            Self::Unicode => &UNICODE_GLYPHS,
            Self::Ascii => &ASCII_GLYPHS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetalView {
    pub column: u16,
    pub row: u16,
    pub glyph: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Petal {
    column: i32,
    row: i32,
    drift: i32,
    speed: i32,
    glyph: char,
}

pub struct Bloom {
    rng: Rng,
    petals: Vec<Petal>,
    glyphs: Glyphs,
}

impl Bloom {
    pub fn new(seed: u64) -> Self {
        Self::with_glyphs(seed, Glyphs::detect())
    }

    pub fn with_glyphs(seed: u64, glyphs: Glyphs) -> Self {
        Self {
            rng: Rng::new(seed),
            petals: Vec::new(),
            glyphs,
        }
    }

    pub fn tick(&mut self, width: u16, height: u16) -> bool {
        if width == 0 || height == 0 {
            return self.clear();
        }
        let target = (usize::from(width) / 20).clamp(2, 8);
        while self.petals.len() < target {
            self.spawn(width, height);
        }
        self.petals.truncate(target);
        let width = i32::from(width) * SUBCELL;
        let height = i32::from(height) * SUBCELL;
        for petal in &mut self.petals {
            petal.column = (petal.column + petal.drift).rem_euclid(width);
            petal.row = (petal.row + petal.speed).rem_euclid(height);
        }
        true
    }

    pub fn clear(&mut self) -> bool {
        let changed = !self.petals.is_empty();
        self.petals.clear();
        changed
    }

    pub fn is_empty(&self) -> bool {
        self.petals.is_empty()
    }

    pub fn petals(&self) -> impl Iterator<Item = PetalView> + '_ {
        self.petals.iter().map(|petal| PetalView {
            column: u16::try_from(petal.column / SUBCELL).unwrap_or(u16::MAX),
            row: u16::try_from(petal.row / SUBCELL).unwrap_or(u16::MAX),
            glyph: petal.glyph,
        })
    }

    fn spawn(&mut self, width: u16, height: u16) {
        let subcell = u32::try_from(SUBCELL).unwrap_or(1);
        let column = self.rng.below(u32::from(width) * subcell);
        let row = self.rng.below(u32::from(height) * subcell);
        let drift = self.rng.below(3);
        let speed = 1 + self.rng.below(3);
        let glyphs = self.glyphs.characters();
        let index = usize::try_from(self.rng.below(3)).unwrap_or(0);
        self.petals.push(Petal {
            column: i32::try_from(column).unwrap_or(0),
            row: i32::try_from(row).unwrap_or(0),
            drift: i32::try_from(drift).unwrap_or(0) - 1,
            speed: i32::try_from(speed).unwrap_or(2),
            glyph: glyphs[index],
        });
    }
}

#[derive(Debug)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> u64 {
        let mut state = self.state;
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        self.state = state;
        state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        u32::try_from(self.next() >> 32).unwrap_or(0) % bound
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Idle {
    last_input: Instant,
}

impl Idle {
    pub fn new(now: Instant) -> Self {
        Self { last_input: now }
    }

    pub fn note_input(&mut self, now: Instant) {
        self.last_input = now;
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.last_input)
    }

    pub fn expired(&self, now: Instant) -> bool {
        self.elapsed(now) >= IDLE_DELAY
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        IDLE_DELAY.saturating_sub(self.elapsed(now))
    }
}

pub fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x5eed_5eed_5eed_5eed, |elapsed| {
            u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use unicode_width::UnicodeWidthChar;

    use super::{
        ASCII_GLYPHS, Bloom, Glyphs, IDLE_DELAY, Idle, Petal, Rng, SUBCELL, UNICODE_GLYPHS,
    };

    fn bloom(seed: u64) -> Bloom {
        Bloom::with_glyphs(seed, Glyphs::Ascii)
    }

    #[test]
    fn every_glyph_has_unambiguous_width() {
        for glyph in UNICODE_GLYPHS.into_iter().chain(ASCII_GLYPHS) {
            assert_eq!(UnicodeWidthChar::width(glyph), Some(1), "glyph {glyph:?}");
        }
    }

    #[test]
    fn ascii_glyphs_are_ascii() {
        assert!(ASCII_GLYPHS.iter().all(char::is_ascii));
    }

    #[test]
    fn locale_without_utf8_uses_ascii_glyphs() {
        assert_eq!(Glyphs::from_locale(None), Glyphs::Ascii);
        assert_eq!(Glyphs::from_locale(Some("C")), Glyphs::Ascii);
        assert_eq!(Glyphs::from_locale(Some("en_US.UTF-8")), Glyphs::Unicode);
    }

    #[test]
    fn same_seed_produces_same_petals() {
        let mut first = bloom(7);
        let mut second = bloom(7);
        for _ in 0..10 {
            first.tick(40, 12);
            second.tick(40, 12);
        }
        assert_eq!(first.petals, second.petals);
    }

    #[test]
    fn different_seeds_produce_different_petals() {
        let mut first = bloom(7);
        let mut second = bloom(8);
        first.tick(40, 12);
        second.tick(40, 12);
        assert_ne!(first.petals, second.petals);
    }

    #[test]
    fn petals_stay_inside_bounds_and_wrap() {
        let mut bloom = bloom(11);
        let width = 30;
        let height = 8;
        for _ in 0..200 {
            bloom.tick(width, height);
            for petal in &bloom.petals {
                assert!((0..i32::from(width) * SUBCELL).contains(&petal.column));
                assert!((0..i32::from(height) * SUBCELL).contains(&petal.row));
            }
        }
    }

    #[test]
    fn petals_wrap_at_every_edge() {
        let mut bloom = Bloom {
            rng: Rng::new(1),
            petals: vec![Petal {
                column: i32::from(30_u16) * SUBCELL - 1,
                row: i32::from(8_u16) * SUBCELL - 1,
                drift: 1,
                speed: 2,
                glyph: '.',
            }],
            glyphs: Glyphs::Ascii,
        };
        bloom.tick(30, 8);
        assert_eq!(bloom.petals[0].column, 0);
        assert_eq!(bloom.petals[0].row, 1);

        bloom.petals[0].column = 0;
        bloom.petals[0].row = 0;
        bloom.petals[0].drift = -1;
        bloom.petals[0].speed = -2;
        bloom.tick(30, 8);
        assert_eq!(bloom.petals[0].column, i32::from(30_u16) * SUBCELL - 1);
        assert_eq!(bloom.petals[0].row, i32::from(8_u16) * SUBCELL - 2);
    }

    #[test]
    fn tick_spawns_at_least_two_petals() {
        let mut bloom = bloom(3);
        assert!(bloom.tick(1, 1));
        assert_eq!(bloom.petals.len(), 2);
    }

    #[test]
    fn zero_area_clears_petals() {
        let mut bloom = bloom(3);
        bloom.tick(10, 10);
        assert!(bloom.tick(0, 10));
        assert!(bloom.is_empty());
    }

    #[test]
    fn clear_reports_whether_petals_existed() {
        let mut bloom = bloom(3);
        assert!(!bloom.clear());
        bloom.tick(20, 8);
        assert!(bloom.clear());
        assert!(!bloom.clear());
    }

    #[test]
    fn petal_view_maps_subcells_to_cells() {
        let bloom = Bloom {
            rng: Rng::new(1),
            petals: vec![Petal {
                column: 3 * SUBCELL + 5,
                row: 2 * SUBCELL + 9,
                drift: 0,
                speed: 0,
                glyph: '*',
            }],
            glyphs: Glyphs::Ascii,
        };
        let view = bloom.petals().next().expect("one petal");
        assert_eq!(view.column, 3);
        assert_eq!(view.row, 2);
        assert_eq!(view.glyph, '*');
    }

    #[test]
    fn idle_expires_only_after_delay() {
        let start = Instant::now();
        let idle = Idle::new(start);
        assert!(!idle.expired(start));
        assert!(!idle.expired(start + IDLE_DELAY.saturating_sub(Duration::from_millis(1))));
        assert!(idle.expired(start + IDLE_DELAY));
        assert_eq!(idle.remaining(start), IDLE_DELAY);
        assert_eq!(idle.remaining(start + IDLE_DELAY), Duration::ZERO);
    }

    #[test]
    fn input_restarts_idle_timer() {
        let start = Instant::now();
        let mut idle = Idle::new(start);
        let later = start + IDLE_DELAY;
        idle.note_input(later);
        assert!(!idle.expired(later));
        assert!(idle.expired(later + IDLE_DELAY));
    }

    #[test]
    fn fixed_seed_positions_after_ten_ticks() {
        let mut bloom = bloom(42);
        for _ in 0..10 {
            bloom.tick(40, 12);
        }
        let positions: Vec<_> = bloom
            .petals
            .iter()
            .map(|petal| (petal.column, petal.row, petal.glyph))
            .collect();
        assert_eq!(positions, vec![(429, 29, '\''), (172, 132, '*')]);
    }
}
