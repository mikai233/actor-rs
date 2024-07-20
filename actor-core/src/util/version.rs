use std::cmp::{min, Ordering};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use anyhow::{anyhow, bail};
use bincode::{Decode, Encode};
use imstr::ImString;
use serde::{Deserialize, Serialize};

const UNDEFINED: i32 = 0;

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    Serialize,
    Deserialize
)]
pub struct Version {
    version: ImString,
    numbers: [i32; 4],
    rest: ImString,
}

impl Version {
    pub fn new(version: impl Into<ImString>) -> anyhow::Result<Self> {
        let version = version.into();
        let (numbers, rest) = Self::parse(&version)?;
        Ok(Version { version, numbers, rest })
    }

    pub fn zero() -> &'static Version {
        static ZERO: OnceLock<Version> = OnceLock::new();
        ZERO.get_or_init(|| Version::new("0.0.0").unwrap())
    }

    fn parse(version: &str) -> anyhow::Result<([i32; 4], ImString)> {
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

        let rst = if segments.len() == 1 {
            let s = segments[0];
            if s.is_empty() {
                bail!("Empty version not supported.");
            }
            numbers[1] = UNDEFINED;
            numbers[2] = UNDEFINED;
            numbers[3] = UNDEFINED;
            let first_char = s.chars().next().unwrap();
            if first_char.is_digit(10) {
                match first_char.to_digit(10) {
                    None => {
                        s
                    }
                    Some(v) => {
                        numbers[0] = v as i32;
                        ""
                    }
                }
            } else {
                s
            }
        } else if segments.len() == 2 {
            let (n1, n2, reset) = parse_last_parts(segments[1]);
            numbers[0] = segments[0].parse().map_err(|_| anyhow!("Invalid version number {}", segments[0]))?;
            numbers[1] = n1;
            numbers[2] = n2;
            numbers[3] = UNDEFINED;
            reset
        } else if segments.len() == 3 {
            let (n1, n2, reset) = parse_last_parts(segments[2]);
            numbers[0] = segments[0].parse().map_err(|_| anyhow!("Invalid version number {}", segments[0]))?;
            numbers[1] = segments[1].parse().map_err(|_| anyhow!("Invalid version number {}", segments[1]))?;
            numbers[2] = n1;
            numbers[3] = n2;
            reset
        } else {
            bail!("Only 3 digits separated with '.' are supported. [{version}]")
        };
        Ok((numbers, rst.into()))
    }

    pub fn numbers(&self) -> &[i32; 4] {
        &self.numbers
    }

    pub fn rest(&self) -> &str {
        &self.rest
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.version == other.version {
            Some(Ordering::Equal)
        } else {
            let mut ordering = self.numbers[0].partial_cmp(&other.numbers[0]);
            if ordering == Some(Ordering::Equal) {
                ordering = self.numbers[1].partial_cmp(&other.numbers[1]);
                if ordering == Some(Ordering::Equal) {
                    ordering = self.numbers[2].partial_cmp(&other.numbers[2]);
                    if ordering == Some(Ordering::Equal) {
                        ordering = self.numbers[3].partial_cmp(&other.numbers[3]);
                        if ordering == Some(Ordering::Equal) {
                            if self.rest == "" && other.rest != "" {
                                ordering = Some(Ordering::Greater);
                            } else if other.rest == "" && self.rest != "" {
                                ordering = Some(Ordering::Less);
                            } else {
                                ordering = self.rest.partial_cmp(&other.rest);
                            }
                        }
                    }
                }
            }
            ordering
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.version == other.version {
            Ordering::Equal
        } else {
            let mut ordering = self.numbers[0].cmp(&other.numbers[0]);
            if ordering == Ordering::Equal {
                ordering = self.numbers[1].cmp(&other.numbers[1]);
                if ordering == Ordering::Equal {
                    ordering = self.numbers[2].cmp(&other.numbers[2]);
                    if ordering == Ordering::Equal {
                        ordering = self.numbers[3].cmp(&other.numbers[3]);
                        if ordering == Ordering::Equal {
                            if self.rest == "" && other.rest != "" {
                                ordering = Ordering::Greater;
                            } else if other.rest == "" && self.rest != "" {
                                ordering = Ordering::Less;
                            } else {
                                ordering = self.rest.cmp(&other.rest);
                            }
                        }
                    }
                }
            }
            ordering
        }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.version.cmp(&other.version) == Ordering::Equal
    }
}

impl Eq for Version {}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.numbers.iter().for_each(|n| n.hash(state));
    }
}