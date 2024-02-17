use std::mem::discriminant;

use crate::{instruction::Instruction, process::State};

/// A breakpoint that can be set on the Intcode computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    MemoryLocation(usize),
    Instruction(Instruction),
}

impl Breakpoint {
    /// Evaluate whether the breakpoint should be triggered.
    pub fn evaluate(&self, state: &State, instruction: &Instruction) -> bool {
        match self {
            Breakpoint::Instruction(i) => discriminant(i) == discriminant(instruction),
            Breakpoint::MemoryLocation(location) => {
                let size = instruction.parameter_count() + 1;
                let start = state.instruction_pointer;
                (start..start + size).contains(location)
            }
        }
    }
}

/// A collection of breakpoints.
#[derive(Default, Clone)]
pub struct Breakpoints {
    breakpoints: Vec<Breakpoint>,
}

impl Breakpoints {
    /// Add a breakpoint to the collection.
    pub fn add(&mut self, breakpoint: Breakpoint) {
        self.breakpoints.push(breakpoint);
    }

    /// Evaluate whether any of the breakpoints should be triggered.
    pub fn evaluate(&self, state: &State, instruction: &Instruction) -> bool {
        self.breakpoints
            .iter()
            .any(|b| b.evaluate(state, instruction))
    }
}

impl IntoIterator for Breakpoints {
    type Item = Breakpoint;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.breakpoints.into_iter()
    }
}
