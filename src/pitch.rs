//! The pitch arithmetic of SuperCollider on numbers.

/// The frequency of the MIDI note `note`, in hertz, as `sc_midicps` of
/// SuperCollider computes it for a float64.
pub(crate) fn midicps(note: f64) -> f64 {
    440.0 * 2.0_f64.powf((note - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::midicps;

    #[test]
    fn midicps_of_60_is_261_6255653005986_and_of_69_is_440() {
        assert_eq!(midicps(60.0).to_bits(), 261.625_565_300_598_6_f64.to_bits());
        assert_eq!(midicps(69.0).to_bits(), 440.0_f64.to_bits());
    }
}
