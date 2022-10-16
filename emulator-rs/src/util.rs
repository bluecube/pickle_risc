use more_asserts::*;

/// Takes `type::BITS` least significant bits and converts them to `type` using
/// unwrapped `TryFrom`.
macro_rules! field {
    ($e:expr, $type:ident) => { $type::try_from((($e) & ((1 << $type::BITS) - 1))).expect("Should fit because of the mask size") }
}
pub(crate) use field;

// TODO: Performance: compiler explorer seems to not inline this. Benchmark with #[inline] later.
/// Takes `bits` least significant bits from `value` and sign extends them to 16 bits.
/// `bits` must be between 1 and 15 (inclusive) (16 bit fields could work, but don't make much
/// sense, so they are not supported)
pub fn sign_extend_field(value: u16, bits: u16) -> u16 {
    assert_ge!(bits, 1);
    assert_le!(bits, 15);
    let top_bit = 1 << (bits - 1);
    let mask = u16::MAX >> (16 - bits);
    let masked_value = value & mask;
    (masked_value ^ top_bit).wrapping_sub(top_bit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use test_strategy::proptest;

    #[test]
    fn test_sign_extend_examples() {
        assert_eq!(sign_extend_field(0b10, 2), 0xfffe);
        assert_eq!(sign_extend_field(0b01, 2), 0b01);
        assert_eq!(sign_extend_field(0xffa1, 2), 0b01);
    }

    /// Test sign extending a field.
    /// The strategy chooses a width of the field between 2 and 15 bits
    #[proptest]
    fn test_sign_extend_complete(
        #[strategy((1u16..=15u16).prop_flat_map(|field_bits| (
            Just(field_bits),
            (-1i16 << (field_bits - 1))..=(((1u16 << (field_bits - 1)) - 1) as i16),
            (0u16..((1u16 << (16 - field_bits)) - 1))
        )))]
        data: (u16, i16, u16)
    ) {
        let (field_bits, field_value, padding) = data;
        let mask = (1u16 << field_bits) - 1;
        let unsigned = field_value as u16;
        let input = (unsigned & mask) | (padding << field_bits);

        assert_eq!(sign_extend_field(input, field_bits), unsigned);
    }
}
