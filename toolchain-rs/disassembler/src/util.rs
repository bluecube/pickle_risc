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
