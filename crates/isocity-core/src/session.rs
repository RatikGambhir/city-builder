//! Sessions own a city and decide how commands reach it.
//!
//! The UI only talks to the [`Session`] trait. Single-player uses
//! [`LocalSession`], which applies commands immediately and runs the
//! simulation clock locally. An online implementation can implement the same
//! trait by forwarding commands to a server/peers and applying the confirmed,
//! ordered stream (the simulation is deterministic given the state's RNG).

use crate::command::{self, Command, CommandResult};
use crate::sim;
use crate::types::GameState;
use serde::{Deserialize, Serialize};

/// A command stamped with the simulation tick it was issued on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SequencedCommand {
    pub seq: u64,
    pub tick: u64,
    pub command: Command,
}

pub trait Session {
    fn state(&self) -> &GameState;
    fn state_mut(&mut self) -> &mut GameState;
    /// Submit a player action. Local sessions apply it right away.
    fn submit(&mut self, cmd: Command) -> CommandResult;
    /// Advance wall-clock time by `dt` seconds; returns ticks simulated.
    fn advance(&mut self, dt: f32) -> u32;
    /// Total simulation ticks since the session started.
    fn ticks(&self) -> u64;
}

/// Milliseconds between simulation ticks for each speed setting.
pub fn tick_interval(speed: u8) -> Option<f32> {
    match speed {
        1 => Some(0.5),
        2 => Some(0.3),
        3 => Some(0.2),
        _ => None,
    }
}

pub struct LocalSession {
    state: GameState,
    accumulator: f32,
    ticks: u64,
    next_seq: u64,
    /// Every applied command, in order. A network layer can drain this and
    /// broadcast it; it also serves as a replay log.
    outbox: Vec<SequencedCommand>,
}

impl LocalSession {
    pub fn new(state: GameState) -> Self {
        Self {
            state,
            accumulator: 0.0,
            ticks: 0,
            next_seq: 0,
            outbox: Vec::new(),
        }
    }

    pub fn into_state(self) -> GameState {
        self.state
    }

    /// Takes the commands applied since the last call.
    pub fn drain_outbox(&mut self) -> Vec<SequencedCommand> {
        std::mem::take(&mut self.outbox)
    }

    /// Runs exactly one simulation tick regardless of speed.
    pub fn step(&mut self) {
        sim::simulate_tick(&mut self.state);
        self.ticks += 1;
    }
}

impl Session for LocalSession {
    fn state(&self) -> &GameState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut GameState {
        &mut self.state
    }

    fn submit(&mut self, cmd: Command) -> CommandResult {
        let result = command::apply(&mut self.state, &cmd);
        if result.is_ok() && !cmd.is_local_only() {
            self.outbox.push(SequencedCommand {
                seq: self.next_seq,
                tick: self.ticks,
                command: cmd,
            });
            self.next_seq += 1;
            if self.outbox.len() > 10_000 {
                self.outbox.drain(..5_000);
            }
        }
        result
    }

    fn advance(&mut self, dt: f32) -> u32 {
        let Some(interval) = tick_interval(self.state.speed) else {
            self.accumulator = 0.0;
            return 0;
        };
        self.accumulator += dt.min(1.0);
        let mut n = 0;
        while self.accumulator >= interval && n < 4 {
            self.accumulator -= interval;
            self.step();
            n += 1;
        }
        if n == 4 {
            self.accumulator = 0.0;
        }
        n
    }

    fn ticks(&self) -> u64 {
        self.ticks
    }
}
