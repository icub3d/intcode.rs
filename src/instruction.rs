use std::fmt::Display;

use crate::parameter::Parameter;

/// An instruction that can be executed by the Intcode computer.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Instruction {
    /// Add two values and store the result in a third.
    Add(Parameter, Parameter, Parameter),
    /// Multiply two values and store the result in a third.
    Multiply(Parameter, Parameter, Parameter),
    /// Read a value from the input channel and store it in memory.
    Input(Parameter),
    /// Write a value to the output channel.
    Output(Parameter),
    /// Jump to a new instruction if the value is non-zero.
    JumpIfTrue(Parameter, Parameter),
    /// Jump to a new instruction if the value is zero.
    JumpIfFalse(Parameter, Parameter),
    /// Store 1 in the third parameter if the first parameter is less than the second parameter,
    /// otherwise store 0.
    LessThan(Parameter, Parameter, Parameter),
    /// Store 1 in the third parameter if the first parameter is equal to the second parameter,
    /// otherwise store 0.
    Equals(Parameter, Parameter, Parameter),
    /// Adjust the relative base.
    AdjustRelativeBaseOffset(Parameter),
    /// Halt the program.
    Halt,
}

impl Instruction {
    /// The list of the names of all the instructions.
    pub const NAMES: [&'static str; 10] = [
        "ADD", "MUL", "INP", "OUT", "JIT", "JIF", "LST", "EQL", "ARO", "HLT",
    ];

    /// Get the number of parameters for a given instruction. This will be used by the tui to
    /// highlight the parameters of an operation. Also useful for incrementing the instruction
    /// pointer.
    pub fn parameter_count(&self) -> usize {
        match self {
            Instruction::Add(_, _, _) => 3,
            Instruction::Multiply(_, _, _) => 3,
            Instruction::Input(_) => 1,
            Instruction::Output(_) => 1,
            Instruction::JumpIfTrue(_, _) => 2,
            Instruction::JumpIfFalse(_, _) => 2,
            Instruction::LessThan(_, _, _) => 3,
            Instruction::Equals(_, _, _) => 3,
            Instruction::AdjustRelativeBaseOffset(_) => 1,
            Instruction::Halt => 0,
        }
    }

    /// Get the parameters in relative mode for a given instruction. This will be used by the tui
    /// to highlight the memory locations that are being read from or written to.
    pub fn relative_parameters(&self, base: isize) -> Vec<usize> {
        let mut relatives = Vec::new();
        macro_rules! add_relatives {
            ($param:ident) => {
                if let Parameter::Relative(offset) = $param {
                    relatives.push((base + *offset) as usize);
                }
            };
            ($param:ident, $($params:ident),+) => {
                add_relatives! { $param }
                add_relatives! { $($params),+ }
            };
        }
        match self {
            Instruction::Add(left, right, dest) => {
                add_relatives! { left, right, dest }
            }
            Instruction::Multiply(left, right, dest) => {
                add_relatives! { left, right, dest }
            }
            Instruction::Input(dest) => {
                add_relatives! { dest }
            }
            Instruction::Output(value) => {
                add_relatives! { value }
            }
            Instruction::JumpIfTrue(value, dest) => {
                add_relatives! { value, dest }
            }
            Instruction::JumpIfFalse(value, dest) => {
                add_relatives! { value, dest }
            }
            Instruction::LessThan(left, right, dest) => {
                add_relatives! { left, right, dest }
            }
            Instruction::Equals(left, right, dest) => {
                add_relatives! { left, right, dest }
            }
            Instruction::AdjustRelativeBaseOffset(value) => {
                add_relatives! { value }
            }
            Instruction::Halt => {}
        }
        relatives
    }

    /// Get the parameters in position mode for a given instruction. This will be used by the tui
    /// to highlight the memory locations that are being read from or written to.
    pub fn position_parameters(&self) -> Vec<usize> {
        let mut positions = Vec::new();
        macro_rules! add_positions {
            ($param:ident) => {
                if let Parameter::Position(pos) = $param {
                    positions.push(*pos);
                }
            };
            ($param:ident, $($params:ident),+) => {
                add_positions! { $param }
                add_positions! { $($params),+ }
            };
        }
        match self {
            Instruction::Add(left, right, dest) => {
                add_positions! { left, right, dest }
            }
            Instruction::Multiply(left, right, dest) => {
                add_positions! { left, right, dest }
            }
            Instruction::Input(dest) => {
                add_positions! { dest }
            }
            Instruction::Output(value) => {
                add_positions! { value }
            }
            Instruction::JumpIfTrue(value, dest) => {
                add_positions! { value, dest }
            }
            Instruction::JumpIfFalse(value, dest) => {
                add_positions! { value, dest }
            }
            Instruction::LessThan(left, right, dest) => {
                add_positions! { left, right, dest }
            }
            Instruction::Equals(left, right, dest) => {
                add_positions! { left, right, dest }
            }
            Instruction::AdjustRelativeBaseOffset(value) => {
                add_positions! { value }
            }
            Instruction::Halt => {}
        }
        positions
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Add(left, right, dest) => {
                write!(f, "ADD {} + {} -> {}", left, right, dest)
            }
            Instruction::Multiply(left, right, dest) => {
                write!(f, "MUL {} * {} -> {}", left, right, dest)
            }
            Instruction::Input(dest) => write!(f, "INP -> {}", dest),
            Instruction::Output(value) => write!(f, "OUT -> {}", value),
            Instruction::JumpIfTrue(value, dest) => write!(f, "JIT {} -> {}", value, dest),
            Instruction::JumpIfFalse(value, dest) => write!(f, "JIF {} -> {}", value, dest),
            Instruction::LessThan(left, right, dest) => {
                write!(f, "LST {} < {} -> {}", left, right, dest)
            }
            Instruction::Equals(left, right, dest) => {
                write!(f, "EQL {} == {} -> {}", left, right, dest)
            }
            Instruction::AdjustRelativeBaseOffset(value) => write!(f, "ARO {}", value),
            Instruction::Halt => write!(f, "HLT"),
        }
    }
}

impl From<&str> for Instruction {
    fn from(s: &str) -> Self {
        match s {
            "ADD" => Instruction::Add(
                Parameter::Position(0),
                Parameter::Position(0),
                Parameter::Position(0),
            ),
            "MUL" => Instruction::Multiply(
                Parameter::Position(0),
                Parameter::Position(0),
                Parameter::Position(0),
            ),
            "INP" => Instruction::Input(Parameter::Position(0)),
            "OUT" => Instruction::Output(Parameter::Position(0)),
            "JIT" => Instruction::JumpIfTrue(Parameter::Position(0), Parameter::Position(0)),
            "JIF" => Instruction::JumpIfFalse(Parameter::Position(0), Parameter::Position(0)),
            "LST" => Instruction::LessThan(
                Parameter::Position(0),
                Parameter::Position(0),
                Parameter::Position(0),
            ),
            "EQL" => Instruction::Equals(
                Parameter::Position(0),
                Parameter::Position(0),
                Parameter::Position(0),
            ),
            "ARO" => Instruction::AdjustRelativeBaseOffset(Parameter::Position(0)),
            "HLT" => Instruction::Halt,
            // TODO, at this point, we could also see if they fit the pattern in Display and parse
            // them.
            _ => panic!("invalid instruction"),
        }
    }
}
