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
        Self {
            title: None,
            fields: Vec::new(),
            bits_per_row: 32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_bits() {
        let f = PacketField {
            start: 0,
            end: 15,
            label: "Port".into(),
        };
        assert_eq!(f.bits(), 16);
    }

    #[test]
    fn single_bit_field() {
        let f = PacketField {
            start: 106,
            end: 106,
            label: "URG".into(),
        };
        assert_eq!(f.bits(), 1);
    }

    #[test]
    fn default_values() {
        let d = PacketDiagram::default();
        assert!(d.title.is_none());
        assert!(d.fields.is_empty());
        assert_eq!(d.bits_per_row, 32);
    }

    #[test]
    fn field_full_row() {
        let f = PacketField {
            start: 0,
            end: 31,
            label: "Data".into(),
        };
        assert_eq!(f.bits(), 32);
    }

    #[test]
    fn multiple_fields() {
        let d = PacketDiagram {
            title: Some("TCP Header".into()),
            bits_per_row: 32,
            fields: vec![
                PacketField {
                    start: 0,
                    end: 15,
                    label: "Src Port".into(),
                },
                PacketField {
                    start: 16,
                    end: 31,
                    label: "Dst Port".into(),
                },
            ],
        };
        assert_eq!(d.fields.len(), 2);
        assert_eq!(d.title.as_deref(), Some("TCP Header"));
        let total_bits: usize = d.fields.iter().map(|f| f.bits()).sum();
        assert_eq!(total_bits, 32);
    }
}
