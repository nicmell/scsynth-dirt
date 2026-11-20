//! The arithmetic of sclang on numbers.

/// The frequency of the MIDI note `note`, in hertz, as `sc_midicps` of
/// SuperCollider computes it for a float64.
pub(crate) fn midicps(note: f64) -> f64 {
    440.0 * 2.0_f64.powf((note - 69.0) / 12.0)
}

/// The amplitude of a level of `db` decibels, as `sc_dbamp` of SuperCollider
/// computes it for a float64.
pub(crate) fn dbamp(db: f64) -> f64 {
    10.0_f64.powf(db * 0.05)
}

/// The base-2 logarithm of the magnitude of `x`, as `sc_log2` of
/// SuperCollider computes it for a float64.
pub(crate) fn log2(x: f64) -> f64 {
    x.abs().log2()
}

#[cfg(test)]
mod tests {
    use super::{dbamp, log2, midicps};

    #[test]
    fn midicps_of_60_is_261_6255653005986_and_of_69_is_440() {
        assert_eq!(midicps(60.0).to_bits(), 261.625_565_300_598_6_f64.to_bits());
        assert_eq!(midicps(69.0).to_bits(), 440.0_f64.to_bits());
    }

    #[test]
    fn dbamp_of_0_is_1_and_log2_of_8_and_of_minus_8_is_3() {
        assert_eq!(dbamp(0.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(log2(8.0).to_bits(), 3.0_f64.to_bits());
        assert_eq!(log2(-8.0).to_bits(), 3.0_f64.to_bits());
    }
}
