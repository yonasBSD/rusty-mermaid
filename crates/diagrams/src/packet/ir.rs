/// Packet diagram: bit-field layout for protocol headers.

#[derive(Debug, Clone)]
pub struct PacketDiagram {
    pub title: Option<String>,
    pub fields: Vec<PacketField>,
    pub bits_per_row: usize,
}

#[derive(Debug, Clone)]
pub struct PacketField {
    pub start: usize,
    pub end: usize, // inclusive
    pub label: String,
}

impl PacketField {
    pub fn bits(&self) -> usize {
        self.end - self.start + 1
    }
}

impl Default for PacketDiagram {
    fn default() -> Self {
        Self { title: None, fields: Vec::new(), bits_per_row: 32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_bits() {
        let f = PacketField { start: 0, end: 15, label: "Port".into() };
        assert_eq!(f.bits(), 16);
    }

    #[test]
    fn single_bit_field() {
        let f = PacketField { start: 106, end: 106, label: "URG".into() };
        assert_eq!(f.bits(), 1);
    }
}
