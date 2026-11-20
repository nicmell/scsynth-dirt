//! The vowels of the Vowel quark at commit `ab59caa`: the formants of the
//! vowels a, e, i, o and u in five voice registers, from `formLib` of
//! `Vowel.sc`.

use crate::sclang;

/// The voice register of the vowels. The default is the counter tenor, as
/// in SuperDirt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum Register {
    /// The soprano.
    Soprano,
    /// The alto.
    Alto,
    /// The tenor.
    Tenor,
    /// The bass.
    Bass,
    /// The counter tenor.
    #[default]
    CounterTenor,
}

/// The five formants of a vowel, as the methods `freqs`, `amps` and `rqs`
/// of `Vowel` give them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Formants {
    /// The frequencies, in hertz.
    pub(crate) freqs: [f64; 5],
    /// The amplitudes: the `dbamp` of the levels.
    pub(crate) amps: [f64; 5],
    /// The reciprocal Qs: the reciprocals of the bandwidths.
    pub(crate) rqs: [f64; 5],
}

/// The formants of the vowel `name` in `register`, or none for a name that
/// is none of a, e, i, o and u.
pub(crate) fn formants(register: Register, name: &str) -> Option<Formants> {
    let vowels = match register {
        Register::Soprano => &SOPRANO,
        Register::Alto => &ALTO,
        Register::Tenor => &TENOR,
        Register::Bass => &BASS,
        Register::CounterTenor => &COUNTER_TENOR,
    };
    let entry = &vowels[VOWELS.iter().position(|&vowel| vowel == name)?];
    Some(Formants {
        freqs: entry.freq.map(f64::from),
        amps: entry.db.map(|db| sclang::dbamp(f64::from(db))),
        rqs: entry.bw.map(|bw| 1.0 / f64::from(bw)),
    })
}

/// The formants of one vowel in one register, as `formLib` writes them.
struct Entry {
    /// The frequencies, in hertz.
    freq: [i16; 5],
    /// The levels, in decibels.
    db: [i16; 5],
    /// The bandwidths, in hertz.
    bw: [i16; 5],
}

/// The vowels of each register of `formLib`, in their order.
const VOWELS: [&str; 5] = ["a", "e", "i", "o", "u"];

/// The vowels of the soprano, in the order of [`VOWELS`].
const SOPRANO: [Entry; 5] = [
    Entry {
        freq: [800, 1150, 2900, 3900, 4950],
        db: [0, -6, -32, -20, -50],
        bw: [80, 90, 120, 130, 140],
    },
    Entry {
        freq: [350, 2000, 2800, 3600, 4950],
        db: [0, -20, -15, -40, -56],
        bw: [60, 100, 120, 150, 200],
    },
    Entry {
        freq: [270, 2140, 2950, 3900, 4950],
        db: [0, -12, -26, -26, -44],
        bw: [60, 90, 100, 120, 120],
    },
    Entry {
        freq: [450, 800, 2830, 3800, 4950],
        db: [0, -11, -22, -22, -50],
        bw: [70, 80, 100, 130, 135],
    },
    Entry {
        freq: [325, 700, 2700, 3800, 4950],
        db: [0, -16, -35, -40, -60],
        bw: [50, 60, 170, 180, 200],
    },
];

/// The vowels of the alto, in the order of [`VOWELS`].
const ALTO: [Entry; 5] = [
    Entry {
        freq: [800, 1150, 2800, 3500, 4950],
        db: [0, -4, -20, -36, -60],
        bw: [80, 90, 120, 130, 140],
    },
    Entry {
        freq: [400, 1600, 2700, 3300, 4950],
        db: [0, -24, -30, -35, -60],
        bw: [60, 80, 120, 150, 200],
    },
    Entry {
        freq: [350, 1700, 2700, 3700, 4950],
        db: [0, -20, -30, -36, -60],
        bw: [50, 100, 120, 150, 200],
    },
    Entry {
        freq: [450, 800, 2830, 3500, 4950],
        db: [0, -9, -16, -28, -55],
        bw: [70, 80, 100, 130, 135],
    },
    Entry {
        freq: [325, 700, 2530, 3500, 4950],
        db: [0, -12, -30, -40, -64],
        bw: [50, 60, 170, 180, 200],
    },
];

/// The vowels of the tenor, in the order of [`VOWELS`].
const TENOR: [Entry; 5] = [
    Entry {
        freq: [650, 1080, 2650, 2900, 3250],
        db: [0, -6, -7, -8, -22],
        bw: [80, 90, 120, 130, 140],
    },
    Entry {
        freq: [400, 1700, 2600, 3200, 3580],
        db: [0, -14, -12, -14, -20],
        bw: [70, 80, 100, 120, 120],
    },
    Entry {
        freq: [290, 1870, 2800, 3250, 3540],
        db: [0, -15, -18, -20, -30],
        bw: [40, 90, 100, 120, 120],
    },
    Entry {
        freq: [400, 800, 2600, 2800, 3000],
        db: [0, -10, -12, -12, -26],
        bw: [40, 80, 100, 120, 120],
    },
    Entry {
        freq: [350, 600, 2700, 2900, 3300],
        db: [0, -20, -17, -14, -26],
        bw: [40, 60, 100, 120, 120],
    },
];

/// The vowels of the bass, in the order of [`VOWELS`].
const BASS: [Entry; 5] = [
    Entry {
        freq: [600, 1040, 2250, 2450, 2750],
        db: [0, -7, -9, -9, -20],
        bw: [60, 70, 110, 120, 130],
    },
    Entry {
        freq: [400, 1620, 2400, 2800, 3100],
        db: [0, -12, -9, -12, -18],
        bw: [40, 80, 100, 120, 120],
    },
    Entry {
        freq: [250, 1750, 2600, 3050, 3340],
        db: [0, -30, -16, -22, -28],
        bw: [60, 90, 100, 120, 120],
    },
    Entry {
        freq: [400, 750, 2400, 2600, 2900],
        db: [0, -11, -21, -20, -40],
        bw: [40, 80, 100, 120, 120],
    },
    Entry {
        freq: [350, 600, 2400, 2675, 2950],
        db: [0, -20, -32, -28, -36],
        bw: [40, 80, 100, 120, 120],
    },
];

/// The vowels of the counter tenor, in the order of [`VOWELS`].
const COUNTER_TENOR: [Entry; 5] = [
    Entry {
        freq: [660, 1120, 2750, 3000, 3350],
        db: [0, -6, -23, -24, -38],
        bw: [80, 90, 120, 130, 140],
    },
    Entry {
        freq: [440, 1800, 2700, 3000, 3300],
        db: [0, -14, -18, -20, -20],
        bw: [70, 80, 100, 120, 120],
    },
    Entry {
        freq: [270, 1850, 2900, 3350, 3590],
        db: [0, -24, -24, -36, -36],
        bw: [40, 90, 100, 120, 120],
    },
    Entry {
        freq: [430, 820, 2700, 3000, 3300],
        db: [0, -10, -26, -22, -34],
        bw: [40, 80, 100, 120, 120],
    },
    Entry {
        freq: [370, 630, 2750, 3000, 3400],
        db: [0, -20, -23, -30, -34],
        bw: [40, 60, 100, 120, 120],
    },
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Register, formants};

    /// The arrays of `vowels.json`, by name, by vowel and by register.
    type Table = BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<f64>>>>;

    /// Each register, with its name in `vowels.json`.
    const REGISTERS: [(Register, &str); 5] = [
        (Register::Soprano, "soprano"),
        (Register::Alto, "alto"),
        (Register::Tenor, "tenor"),
        (Register::Bass, "bass"),
        (Register::CounterTenor, "counterTenor"),
    ];

    /// The bits of each number as an f32, the type of a control value on the
    /// wire.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the test compares the numbers as f32"
    )]
    fn bits(numbers: &[f64]) -> Vec<u32> {
        numbers
            .iter()
            .map(|&number| (number as f32).to_bits())
            .collect()
    }

    #[test]
    fn each_vowel_of_each_register_has_the_formants_of_vowels_json() {
        let table: Table = serde_json::from_str(include_str!("../tests/fixtures/vowels.json"))
            .expect("vowels.json is JSON");
        for (register, key) in REGISTERS {
            for vowel in ["a", "e", "i", "o", "u"] {
                let formants = formants(register, vowel).unwrap_or_else(|| panic!("{key} {vowel}"));
                let expected = &table[key][vowel];
                for (name, numbers) in [
                    ("freqs", formants.freqs),
                    ("amps", formants.amps),
                    ("rqs", formants.rqs),
                ] {
                    assert_eq!(
                        bits(&numbers),
                        bits(&expected[name]),
                        "{key} {vowel} {name}: {numbers:?} against {:?}",
                        expected[name]
                    );
                }
            }
        }
    }

    #[test]
    fn a_name_that_is_no_vowel_has_no_formants() {
        for name in ["", "A", "y", "ae"] {
            assert_eq!(formants(Register::default(), name), None, "{name:?}");
        }
    }
}
