use more_asserts::*;

/// Interpret `bits` low bits from `v` as an unsigned value and
/// try converting to `T`. Panics if this fails.
pub fn field<T>(v: u16, bits: u32) -> T
where
    T: TryFrom<u16>,
    <T as TryFrom<u16>>::Error: std::fmt::Debug,
{
    let mask = u16::MAX >> (16 - bits);
    T::try_from(v & mask).unwrap()
}

/// Interpret `bits` low bits from `v` as a signed value and
/// try converting to `T`. Panics if this fails.
pub fn sign_extend_field<T>(v: u16, bits: u32) -> T
where
    T: TryFrom<i16>,
    <T as TryFrom<i16>>::Error: std::fmt::Debug,
{
    assert_ge!(bits, 1);
    assert_le!(bits, 15);
    let top_bit = 1 << (bits - 1);
    let mask = u16::MAX >> (16 - bits);
    let masked_value = v & mask;
    T::try_from(if masked_value & top_bit != 0 {
        (masked_value as i16) - (mask as i16) - 1
    } else {
        masked_value as i16
    })
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use ux::*;
    use test_strategy::proptest;

    #[test]
    fn test_sign_extend_examples() {
        assert_eq!(sign_extend_field::<i2>(0b10, 2), i2::new(-2));
        assert_eq!(sign_extend_field::<i2>(0b01, 2), i2::new(1));
        assert_eq!(sign_extend_field::<i2>(0xffa1, 2), i2::new(1));
        assert_eq!(sign_extend_field::<i7>(0b1111111, 7), i7::new(-1));
    }

    /// Test sign extending a field.
    /// The strategy chooses a width of the field between 1 and 15 bits
    #[proptest]
    fn test_sign_extend_complete(
        #[strategy((1u32..=15u32).prop_flat_map(|field_bits| (
            Just(field_bits),
            (-1i16 << (field_bits - 1))..(((1u16 << (field_bits - 1)) - 1) as i16),
            (0u16..((1u16 << (16 - field_bits)) - 1))
        )))]
        data: (u32, i16, u16),
    ) {
        let (field_bits, field_value, padding) = data;
        let mask = (1u16 << field_bits) - 1;
        let unsigned = field_value as u16;
        let input = (unsigned & mask) | (padding << field_bits);

        assert_eq!(sign_extend_field::<i16>(input, field_bits), field_value);
    }

    #[proptest]
    fn test_field_u8(n: u16) {
        assert_eq!(field::<u8>(n, 8), (n & 0xff) as u8);
    }

    #[proptest]
    fn test_field_complete(
        #[strategy((1u32..=15u32).prop_flat_map(|field_bits| (
            Just(field_bits),
            0u16..(1u16 << field_bits),
            0u16..(1u16 << (16 - field_bits)),
        )))]
        data: (u32, u16, u16),
    ) {
        let (field_bits, field_value, padding) = data;
        let mask = (1u16 << field_bits) - 1;
        let input = (field_value & mask) | (padding << field_bits);

        assert_eq!(field::<u16>(input, field_bits), field_value);
    }
}
