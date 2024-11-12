use std::{
    fmt::{self, Display},
    str::FromStr,
    sync::OnceLock,
};

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::errors::DenomError;

// Denominations can be 3 ~ 128 characters long and support letters, followed by either
// a letter, a number or a separator ('/').
pub fn regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();

    RE.get_or_init(|| {
        Regex::new(r"^[a-zA-Z][a-zA-Z0-9/-]{2,127}$").expect("hard coded RE won't fail")
    })
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Eq, Hash, PartialOrd, Ord)]
pub struct Denom(String);

impl Denom {
    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Denom {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<[u8]> for Denom {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl TryFrom<String> for Denom {
    type Error = DenomError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        if !regex().is_match(&v) {
            return Err(DenomError);
        };

        Ok(Denom(v))
    }
}

impl TryFrom<&str> for Denom {
    type Error = DenomError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        if !regex().is_match(v) {
            return Err(DenomError);
        };

        Ok(Denom(v.to_string()))
    }
}

impl FromStr for Denom {
    type Err = DenomError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_string())
    }
}

impl From<Denom> for String {
    fn from(value: Denom) -> Self {
        value.0
    }
}

impl Display for Denom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {

    use extensions::testing::UnwrapTesting;

    use super::*;

    #[test]
    fn from_string_successes() {
        let res: Denom = "abcd".to_string().try_into().unwrap_test();
        assert_eq!(Denom("abcd".into()), res);

        let res: Denom = "ibc/7F1D3FCF4AE79E1554D670D1AD949A9BA4E4A3C76C63093E17E446A46061A7A2"
            .to_string()
            .try_into()
            .unwrap_test();
        assert_eq!(
            Denom("ibc/7F1D3FCF4AE79E1554D670D1AD949A9BA4E4A3C76C63093E17E446A46061A7A2".into()),
            res
        );

        let res: Denom = "at0m".to_string().try_into().unwrap_test();
        assert_eq!(Denom("at0m".into()), res);

        let res: Denom = "Atom".to_string().try_into().unwrap_test();
        assert_eq!(Denom("Atom".into()), res);
    }

    #[test]
    fn from_string_failures() {
        // too short
        let res: Result<Denom, DenomError> = "a".to_owned().try_into();
        let err = res.unwrap_err();
        assert_eq!(err, DenomError);

        // starts with a number
        let res: Result<Denom, DenomError> = "8aaaaaaaaaaa".to_owned().try_into();
        let err = res.unwrap_err();
        assert_eq!(err, DenomError);

        // too long
        let res: Result<Denom, DenomError> = "abcdefghijklmnopqrstuvwxyzxxxxxxxxxxabcdefghijklmnopqrstuvwxyzxxxxxxxxxxabcdefghijklmnopqrstuvwxyzxxxxxxxxxx123456789012345678901".to_owned().try_into();
        let err = res.unwrap_err();
        assert_eq!(err, DenomError);

        // non alpha numeric character
        let res: Result<Denom, DenomError> = "ab🙂cd".to_owned().try_into();
        let err = res.unwrap_err();
        assert_eq!(err, DenomError);

        // non alpha numeric characters
        let res: Result<Denom, DenomError> = "     ".to_owned().try_into();
        let err = res.unwrap_err();
        assert_eq!(err, DenomError);

        // non alpha numeric characters
        let res: Result<Denom, DenomError> = "sdsdsd dsdsd".to_owned().try_into();
        let err = res.unwrap_err();
        assert_eq!(err, DenomError);
    }

    #[test]
    fn to_string_success() {
        let denom: Denom = "atom".to_owned().try_into().unwrap_test();
        assert_eq!("atom", denom.to_string());
    }

    #[test]
    fn serialize_success() {
        let res: Denom = "abcd".to_owned().try_into().unwrap_test();

        assert_eq!(
            serde_json::to_string(&res).unwrap_test(),
            r#""abcd""#.to_owned()
        );
    }
}
