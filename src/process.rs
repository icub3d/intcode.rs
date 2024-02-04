use std::collections::HashMap;

use crate::instruction::Instruction;
use crate::ipc::{ChannelReceiver, ChannelSender};
use crate::parameter::Parameter;

use anyhow::Result;

/// The state of the Intcode computer.
#[derive(Debug, Clone)]
pub struct State {
    /// The memory of the computer.
    pub memory: Vec<isize>,
    /// Any additional memory that the computer can use.
    pub additional_memory: HashMap<usize, isize>,
    /// The current instruction pointer.
    pub instruction_pointer: usize,
    /// The current relative base.
    pub relative_base: isize,
    /// The last output value sent to the output channel.
    pub last_output: Option<isize>,
    /// The last input value received from the input channel.
    pub last_input: Option<isize>,
    /// Whether the computer has halted.
    pub halted: bool,
}

impl std::ops::Index<usize> for State {
    type Output = isize;

    fn index(&self, index: usize) -> &Self::Output {
        if index < self.memory.len() {
            &self.memory[index]
        } else {
            self.additional_memory.get(&index).unwrap_or(&0)
        }
    }
}

impl std::ops::IndexMut<usize> for State {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < self.memory.len() {
            &mut self.memory[index]
        } else {
            self.additional_memory.entry(index).or_insert(0)
        }
    }
}

impl State {
    fn new(memory: Vec<isize>) -> Self {
        Self {
            memory,
            additional_memory: HashMap::new(),
            instruction_pointer: 0,
            relative_base: 0,
            last_output: None,
            last_input: None,
            halted: false,
        }
    }

    /// Check if the memory is empty.
    pub fn is_empty(&self) -> bool {
        self.memory.is_empty() && self.additional_memory.is_empty()
    }

    /// Get the length of the memory.
    pub fn len(&self) -> usize {
        self.memory.len() + self.additional_memory.len()
    }

    /// Get the next instruction and the size of the instruction. If there are no more instructions
    /// or the computer has halted, then this will return `None`.
    pub fn next_instruction(&self) -> Option<(Instruction, usize)> {
        if self.instruction_pointer >= self.len() || self.halted {
            return None;
        }

        // Get the opcode and the first two digits (the operation).
        let opcode = self[self.instruction_pointer];
        let op = opcode % 100;

        // This macro simplifies creating the parameters for the instruction.
        macro_rules! param {
            ($instruction:expr, 3) => {
                $instruction(
                    Parameter::new(opcode, 1, self[self.instruction_pointer + 1]),
                    Parameter::new(opcode, 2, self[self.instruction_pointer + 2]),
                    Parameter::new(opcode, 3, self[self.instruction_pointer + 3]),
                )
            };
            ($instruction:expr, 2) => {
                $instruction(
                    Parameter::new(opcode, 1, self[self.instruction_pointer + 1]),
                    Parameter::new(opcode, 2, self[self.instruction_pointer + 2]),
                )
            };
            ($instruction:expr, 1) => {
                $instruction(Parameter::new(
                    opcode,
                    1,
                    self[self.instruction_pointer + 1],
                ))
            };
            ($instruction:expr) => {
                $instruction
            };
        }

        // Create the instruction based on the opcode.
        let instruction = match op {
            1 => param!(Instruction::Add, 3),
            2 => param!(Instruction::Multiply, 3),
            3 => param!(Instruction::Input, 1),
            4 => param!(Instruction::Output, 1),
            5 => param!(Instruction::JumpIfTrue, 2),
            6 => param!(Instruction::JumpIfFalse, 2),
            7 => param!(Instruction::LessThan, 3),
            8 => param!(Instruction::Equals, 3),
            9 => param!(Instruction::AdjustRelativeBaseOffset, 1),
            99 => Instruction::Halt,
            _ => panic!("invalid opcode"),
        };

        Some((instruction, instruction.parameter_count() + 1))
    }
}

/// A process that runs an Intcode program.
pub struct Process {
    state: State,
    channel_receiver: ChannelReceiver,
    channel_sender: ChannelSender,
}

impl Process {
    /// Create a new process with the given program. The receiver will act as the input and the
    /// sender will act as the output.
    pub fn new(
        program: &str,
        channel_receiver: ChannelReceiver,
        channel_sender: ChannelSender,
    ) -> Self {
        Self {
            state: State::new(
                program
                    .trim()
                    .split(',')
                    .map(|s| s.parse::<isize>().unwrap())
                    .collect::<Vec<_>>(),
            ),
            channel_receiver,
            channel_sender,
        }
    }

    /// Receive a value from the input channel. Some programs expect to have one last value that
    /// needs to be read for the solution. This helps with that.
    pub async fn recv(&mut self) -> Option<isize> {
        self.channel_receiver.recv().await
    }

    /// Set the memory at the given index to the given value.
    pub fn set_memory(&mut self, index: usize, value: isize) {
        self.state[index] = value;
    }

    /// Get a copy of the current state of this process.
    pub fn state(&self) -> State {
        self.state.clone()
    }

    /// Run the process until it halts.
    pub async fn run(&mut self) -> Result<()> {
        while !self.state().halted {
            self.step().await?;
        }
        Ok(())
    }

    // Run a single step of the process. If the process successfully ran the instruction, then the
    // instruction pointer will be incremented.
    pub async fn step(&mut self) -> Result<()> {
        if let Some((instruction, instruction_size)) = self.state.next_instruction() {
            match self.evaluate_instruction(instruction).await {
                Ok(true) => self.state.instruction_pointer += instruction_size,
                Ok(false) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    async fn evaluate_instruction(&mut self, instruction: Instruction) -> Result<bool> {
        // If the process is halted, then we don't want to run any more instructions.
        if self.state.halted {
            return Ok(false);
        }

        // This macro simplifies evaluating the parameters for the instruction.
        macro_rules! eval {
            (write $dest:ident) => {
                let $dest = match $dest {
                   Parameter::Position(pos) => pos,
                   Parameter::Relative(offset) => (self.state.relative_base + offset) as usize,
                   Parameter::Immediate(_) => panic!("invalid write parameter"),
                };
            };
            ($param:ident) => {
                let $param = match $param {
                    Parameter::Position(pos) => self.state[pos],
                    Parameter::Relative(offset) => self.state[(self.state.relative_base + offset) as usize],
                    Parameter::Immediate(value) => value,
                };
            };
            ($param:ident, $($params:ident),+) => {
                eval! { $param }
                eval! { $($params),+ }
            };
            (write $dest:ident, $($params:ident),+) => {
                eval! { write $dest }
                eval! { $($params),+ }
            };
        }

        match instruction {
            Instruction::Add(left, right, dest) => {
                eval! { write dest, left, right };
                self.state[dest] = left + right;
            }
            Instruction::Multiply(left, right, dest) => {
                eval! { write dest, left, right };
                self.state[dest] = left * right;
            }
            Instruction::LessThan(left, right, dest) => {
                eval! { write dest, left, right };
                self.state[dest] = match left < right {
                    true => 1,
                    false => 0,
                }
            }
            Instruction::Input(dest) => {
                eval! { write dest };
                self.state[dest] = match self.channel_receiver.recv().await {
                    Some(value) => value,
                    None => return Ok(false),
                };

                self.state.last_input = Some(self.state[dest]);
            }
            Instruction::Output(value) => {
                eval! { value };
                self.state.last_output = Some(value);
                match self.channel_sender.send(value).await {
                    Ok(_) => (),
                    Err(_) => return Ok(false),
                }
            }
            Instruction::JumpIfTrue(value, dest) => {
                eval! { value, dest };
                if value != 0 {
                    self.state.instruction_pointer = dest as usize;
                    // We don't want to update the instruction pointer.
                    return Ok(false);
                }
            }
            Instruction::JumpIfFalse(value, dest) => {
                eval! { value, dest };
                if value == 0 {
                    self.state.instruction_pointer = dest as usize;
                    // We don't want to update the instruction pointer.
                    return Ok(false);
                }
            }
            Instruction::Equals(left, right, dest) => {
                eval! { write dest, left, right };
                self.state[dest] = match left == right {
                    true => 1,
                    false => 0,
                }
            }
            Instruction::AdjustRelativeBaseOffset(value) => {
                eval! { value };
                self.state.relative_base += value;
            }
            Instruction::Halt => {
                self.state.halted = true;
            }
        };
        Ok(true)
    }
}
