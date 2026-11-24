//! Pachinko (D32): a note gate a track's notes are routed through, after the random
//! gates of a well-known eurorack module (and its clones) that drop each clock tick's
//! "ball" left or right.
//!
//! Each unit drops a ball every step (a semiquaver, quaver, crotchet or bar). The ball
//! lands left with the **chance** set, else right. A track routed to a unit's left
//! output plays the notes struck in a step whose ball fell left; one routed to its right
//! output, those whose ball fell right. So one track on the left plays about `chance` of
//! its steps, and two tracks on either side take turns, never both at once. **Déjà vu**
//! makes the balls repeat: at 0 each is new, at 1 the last **length** of them loop for
//! ever, and between, now and then one is replaced. Notes struck in one step (a chord)
//! share its ball.
//!
//! The balls are a function of the step's number, the seed, déjà vu and length alone
//! (worked out from the first step), so a piece plays the same notes live, after a seek,
//! rendered and exported. Turning the chance while a pattern loops thins or fills that
//! same pattern, as the module's bias does.

use serde::{Deserialize, Serialize};

use crate::project::{Note, PPQ, Project};

/// The number of units.
pub const UNITS: usize = 2;
/// The longest loop, in balls.
pub const MAX_LENGTH: usize = 16;

/// How often a ball drops.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rate {
    Sixteenth,
    Eighth,
    Quarter,
    Bar,
}

impl Rate {
    pub const ALL: [Rate; 4] = [Rate::Sixteenth, Rate::Eighth, Rate::Quarter, Rate::Bar];

    pub fn label(self) -> &'static str {
        match self {
            Rate::Sixteenth => "1/16",
            Rate::Eighth => "1/8",
            Rate::Quarter => "1/4",
            Rate::Bar => "Bar",
        }
    }

    /// The step, in ticks.
    pub fn ticks(self, p: &Project) -> u32 {
        match self {
            Rate::Sixteenth => PPQ / 4,
            Rate::Eighth => PPQ / 2,
            Rate::Quarter => PPQ,
            Rate::Bar => p.bar_ticks(),
        }
        .max(1)
    }
}

/// One unit's settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pachinko {
    /// The chance a ball lands left, 0..1.
    pub chance: f32,
    /// How often a ball is one from the loop rather than new, 0..1.
    pub dejavu: f32,
    /// The loop's length in balls, 1..=16.
    pub length: u8,
    pub rate: Rate,
    pub seed: u32,
}

impl Default for Pachinko {
    fn default() -> Self {
        Pachinko { chance: 0.5, dejavu: 0.0, length: 8, rate: Rate::Sixteenth, seed: 1 }
    }
}

/// Which side of which unit a track's notes go through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    pub unit: u8,
    /// The right output (else the left).
    pub right: bool,
}

impl Route {
    pub fn label(self) -> String {
        format!("{} {}", ["A", "B", "C", "D"][self.unit as usize % 4], if self.right { "R" } else { "L" })
    }
}

/// Whether every unit is as it starts (so it's left out of a saved file).
pub fn all_default(u: &[Pachinko; UNITS]) -> bool {
    u.iter().all(|p| *p == Pachinko::default())
}

/// The balls, one a step, as they fall: kept by the engine and moved forward as it plays,
/// allocation-free; worked out again from the first step after a jump back or a change.
#[derive(Clone, Copy, Debug)]
pub struct Balls {
    cfg: (u32, u32, u8),
    rng: u64,
    mem: [f32; MAX_LENGTH],
    /// The next step to drop, and the last ball dropped.
    next: u64,
    last: f32,
}

impl Balls {
    pub fn new(p: &Pachinko) -> Balls {
        let mut b = Balls { cfg: (0, 0, 0), rng: 0, mem: [0.0; MAX_LENGTH], next: 0, last: 0.0 };
        b.reset(p);
        b
    }

    fn key(p: &Pachinko) -> (u32, u32, u8) {
        (p.seed, p.dejavu.clamp(0.0, 1.0).to_bits(), p.length.clamp(1, MAX_LENGTH as u8))
    }

    fn reset(&mut self, p: &Pachinko) {
        self.cfg = Balls::key(p);
        self.rng = (p.seed as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x5851_f42d_4c95_7f2d;
        for k in 0..MAX_LENGTH {
            self.mem[k] = self.unit();
        }
        self.next = 0;
        self.last = 0.0;
    }

    fn unit(&mut self) -> f32 {
        // xorshift64*
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        (self.rng.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40) as f32 / (1u64 << 24) as f32
    }

    /// The ball of step `step` (0..1: left below the chance).
    pub fn at(&mut self, p: &Pachinko, step: u64) -> f32 {
        if Balls::key(p) != self.cfg || step + 1 < self.next {
            self.reset(p);
        }
        let len = p.length.clamp(1, MAX_LENGTH as u8) as u64;
        let dejavu = p.dejavu.clamp(0.0, 1.0);
        while self.next <= step {
            let (keep, fresh) = (self.unit(), self.unit());
            let slot = (self.next % len) as usize;
            if keep >= dejavu {
                self.mem[slot] = fresh;
            }
            self.last = self.mem[slot];
            self.next += 1;
        }
        self.last
    }

    /// Whether a note struck in `step` is heard on `route`'s side.
    pub fn passes(&mut self, p: &Pachinko, step: u64, right: bool) -> bool {
        (self.at(p, step) < p.chance.clamp(0.0, 1.0)) != right
    }
}

/// Which of a track's notes its route lets through, in order (all of them unrouted).
pub fn passed(p: &Project, track: usize, notes: &[Note]) -> Vec<bool> {
    let Some(route) = p.tracks.get(track).and_then(|t| t.route) else { return vec![true; notes.len()] };
    let Some(unit) = p.pachinko.get(route.unit as usize) else { return vec![true; notes.len()] };
    let step = unit.rate.ticks(p) as u64;
    let mut balls = Balls::new(unit);
    // (in time order, so the balls are worked out once)
    let mut order: Vec<usize> = (0..notes.len()).collect();
    order.sort_by_key(|&i| notes[i].start);
    let mut out = vec![true; notes.len()];
    for i in order {
        out[i] = balls.passes(unit, notes[i].start as u64 / step, route.right);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: &Pachinko, n: u64) -> Vec<f32> {
        let mut b = Balls::new(p);
        (0..n).map(|s| b.at(p, s)).collect()
    }

    /// The same settings drop the same balls, however they are asked for.
    #[test]
    fn balls_are_the_same_every_time() {
        let p = Pachinko { dejavu: 0.4, length: 5, seed: 9, ..Pachinko::default() };
        let a = run(&p, 200);
        let mut b = Balls::new(&p);
        // out of order: forwards, back, a jump
        assert_eq!(b.at(&p, 150), a[150]);
        assert_eq!(b.at(&p, 3), a[3]);
        assert_eq!(b.at(&p, 3), a[3]);
        assert_eq!(b.at(&p, 199), a[199]);
        assert!(a.iter().all(|x| (0.0..1.0).contains(x)));
    }

    /// No déjà vu: new balls, about `chance` of them left. Full déjà vu: a loop.
    #[test]
    fn dejavu_loops_the_balls() {
        let p = Pachinko { chance: 0.3, ..Pachinko::default() };
        let a = run(&p, 4000);
        let left = a.iter().filter(|x| **x < 0.3).count() as f64 / 4000.0;
        assert!((0.27..0.33).contains(&left), "{left}");
        assert!(a.windows(8).filter(|w| w[0] == w[7]).count() < 10);
        let p = Pachinko { dejavu: 1.0, length: 6, ..p };
        let a = run(&p, 60);
        assert!(a.windows(7).all(|w| w[0] == w[6]));
        let p = Pachinko { dejavu: 0.8, length: 6, ..p };
        let a = run(&p, 600);
        let same = a.windows(7).filter(|w| w[0] == w[6]).count() as f64 / 594.0;
        assert!((0.7..0.9).contains(&same), "{same}");
    }

    /// Left and right take turns: a step's notes go one way or the other.
    #[test]
    fn left_and_right_take_turns() {
        let p = Pachinko { chance: 0.6, seed: 4, ..Pachinko::default() };
        let mut b = Balls::new(&p);
        for s in 0..500 {
            assert_ne!(b.passes(&p, s, false), b.passes(&p, s, true));
        }
    }
}
