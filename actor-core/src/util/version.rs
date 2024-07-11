use std::cmp::min;
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;

use anyhow::bail;
use bincode::{Decode, Encode};
use imstr::ImString;
use serde::{Deserialize, Serialize};

const UNDEFINED: i32 = 0;

#[derive(
    Debug,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Encode,
    Decode,
    Serialize,
    Deserialize
)]
pub struct Version {
    version: ImString,
    numbers: [i32; 4],
}

impl Version {
    pub fn new(version: impl Into<ImString>) -> Self {
        todo!("Implement Version::new")
    }

    pub fn zero() -> &'static Version {
        static ZERO: OnceLock<Version> = OnceLock::new();
        ZERO.get_or_init(|| Version::new("0.0.0"))
    }

    fn parse(version: &str) -> anyhow::Result<[i32; 4]> {
        fn parse_last_part(s: &str) -> (i32, &str) {
            if s.is_empty() {
                (UNDEFINED, s)
            } else {
                let i = s.find('-');
                let j = s.find('+');
                let k = match (i, j) {
                    (None, j) => {
                        j
                    }
                    (i, None) => {
                        i
                    }
                    (Some(i), Some(j)) => {
                        Some(min(i, j))
                    }
                };
                match k {
                    None => {
                        (s.parse().unwrap(), "")
                    }
                    Some(k) => {
                        (s[..k].parse().unwrap(), &s[(k + 1)..])
                    }
                }
            }
        }

        fn parse_dynver_part(s: &str) -> (i32, &str) {
            if s.is_empty() || !s.chars().next().unwrap().is_digit(10) {
                (UNDEFINED, s)
            } else {
                match s.find('-') {
                    None => {
                        (UNDEFINED, s)
                    }
                    Some(i) => {
                        match s[..i].parse::<i32>() {
                            Ok(num) => {
                                (num, &s[(i + 1)..])
                            }
                            Err(_) => {
                                (UNDEFINED, s)
                            }
                        }
                    }
                }
            }
        }

        fn parse_last_parts(s: &str) -> (i32, i32, &str) {
            let (last_number, reset) = parse_last_part(s);
            if reset == "" {
                (last_number, UNDEFINED, reset)
            } else {
                let (dynver_number, reset2) = parse_dynver_part(reset);
                (last_number, dynver_number, reset2)
            }
        }

        let mut numbers = [UNDEFINED; 4];
        let segments = version.split('.').collect::<Vec<_>>();

        if segments.len() == 1 {
            let s = segments[0];
            if s.is_empty() {
                bail!("Empty version not supported.");
            }
            numbers[1] = UNDEFINED;
            numbers[2] = UNDEFINED;
            numbers[3] = UNDEFINED;
        }
        todo!("Implement Version::parse")
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}