use std::fmt::Display;

/// A parameter to an instruction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Parameter {
    /// A pointer to a position in memory.
    Position(usize),
    /// A literal value.
    Immediate(isize),
    /// A relative pointer to a position in memory.
    Relative(isize),
}

impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Parameter::Position(pos) => write!(f, "P[{}]", pos),
            Parameter::Immediate(value) => write!(f, "I[{}]", value),
            Parameter::Relative(offset) => write!(f, "R[{}]", offset),
        }
    }
}

impl Parameter {
    /// Create a new parameter from an opcode, position, and value. It will use the opcode and
    /// position to determine the parameter mode.
    pub fn new(opcode: isize, position: isize, value: isize) -> Self {
        let mode = (opcode / 10_isize.pow(position as u32 + 1)) % 10;
        match mode {
            0 => Self::Position(value as usize),
            1 => Self::Immediate(value),
            2 => Self::Relative(value),
            _ => panic!("Invalid parameter mode"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parameter_new() {
        // Does the math check out?
        assert_eq!(Parameter::new(1002, 1, 4), Parameter::Position(4));
        assert_eq!(Parameter::new(1002, 2, 3), Parameter::Immediate(3));
        assert_eq!(Parameter::new(1002, 3, 2), Parameter::Position(2));
    }
}
