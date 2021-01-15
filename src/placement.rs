trait Codeword {
    /// A single cell used to encode one bit
    type Module;
    const WHITE: Self::Module;
    const BLACK: Self::Module;

    fn bits(&self) -> [Self::Module; 8];
}

impl Codeword for u8 {
    type Module = bool;
    const WHITE: bool = false;
    const BLACK: bool = true;

    fn bits(&self) -> [bool; 8] {
        [
            (self & 0b1000_0000) != 0,
            (self & 0b0100_0000) != 0,
            (self & 0b0010_0000) != 0,
            (self & 0b0001_0000) != 0,
            (self & 0b0000_1000) != 0,
            (self & 0b0000_0100) != 0,
            (self & 0b0000_0010) != 0,
            (self & 0b0000_0001) != 0,
        ]
    }
}

struct MatrixMap<T: Codeword, const N: usize> {
    entries: [[T::Module; N]; N],
}

#[cfg(test)]
mod tests {
    enum MockModule {
        B,
        W,
        X(String),
    }

    struct MockCodeword(usize);

    impl super::Codeword for MockCodeword {
        type Module = MockModule;
        const WHITE: MockModule = MockModule::W;
        const BLACK: MockModule = MockModule::B;

        fn bits(&self) -> [MockModule; 8] {
            [
                MockModule::X(format!("{}.1", self.0)),
                MockModule::X(format!("{}.2", self.0)),
                MockModule::X(format!("{}.3", self.0)),
                MockModule::X(format!("{}.4", self.0)),
                MockModule::X(format!("{}.5", self.0)),
                MockModule::X(format!("{}.6", self.0)),
                MockModule::X(format!("{}.7", self.0)),
                MockModule::X(format!("{}.8", self.0)),
            ]
        }
    }
}
