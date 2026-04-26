#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Digit {
    Zero = 0,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

impl Digit {
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Zero),
            1 => Some(Self::One),
            2 => Some(Self::Two),
            3 => Some(Self::Three),
            4 => Some(Self::Four),
            5 => Some(Self::Five),
            6 => Some(Self::Six),
            7 => Some(Self::Seven),
            8 => Some(Self::Eight),
            9 => Some(Self::Nine),
            _ => None,
        }
    }
}

pub type PackedDigit = [u8; 3];

pub enum Glyph {
    Digit(Digit),
    Space,
}

impl Glyph {
    #[must_use]
    pub const fn width(&self) -> usize {
        match self {
            Self::Digit(_) => 3,
            Self::Space => 1,
        }
    }
}

impl<'a> From<&'a Digit> for &'a PackedDigit {
    fn from(d: &Digit) -> Self {
        match d {
            Digit::Zero => &[0b0111_1110, 0b1000_0001, 0b0111_1110],
            Digit::One => &[0b0100_0001, 0b1111_1111, 0b0000_0001],
            Digit::Two => &[0b0100_0011, 0b1000_1101, 0b0111_0001],
            Digit::Three => &[0b0100_0001, 0b1001_0001, 0b0110_1110],
            Digit::Four => &[0b1111_1000, 0b0000_1000, 0b1111_1111],
            Digit::Five => &[0b1111_0010, 0b1001_0001, 0b1000_1110],
            Digit::Six => &[0b0111_1110, 0b1000_1001, 0b0100_1110],
            Digit::Seven => &[0b1000_0000, 0b1001_1111, 0b1110_0000],
            Digit::Eight => &[0b0110_1110, 0b1001_0001, 0b0110_1110],
            Digit::Nine => &[0b0110_0010, 0b1001_0001, 0b0111_1110],
        }
    }
}
