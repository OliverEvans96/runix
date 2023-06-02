use std::borrow::Cow;
use std::fmt::Display;
use std::path::{PathBuf, Component};
use std::str::FromStr;

use chrono::{NaiveDateTime, TimeZone, Utc};
use derive_more::{Display, From};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use url::Url;

use self::file::application::ApplicationProtocol;
use self::file::{FileRef, TarballRef, application};
use self::git::GitRef;
use self::git_service::{service, GitServiceRef};
use self::indirect::IndirectRef;
use self::path::PathRef;
use self::protocol::Protocol;

pub mod file;
pub mod git;
pub mod git_service;
pub mod indirect;
pub mod lock;
pub mod path;
pub mod protocol;

pub static FLAKE_ID_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("[a-zA-Z][a-zA-Z0-9_-]*").unwrap());


pub trait FlakeRefSource:  FromStr + Display {
    type ParseErr;

    fn scheme() -> Cow<'static, str>;

    fn from_url(url: Url) -> Result<Self, Self::ParseErr>;

    fn parses(maybe_ref: &str) -> bool {
        maybe_ref.starts_with(&format!("{}:", Self::scheme()))
    }
}



pub trait FromUrl {

}

pub enum ImpureFlakeRef {
    Pure(FlakeRef),
    Impure(String),
}

impl FromStr for ImpureFlakeRef {
    type Err = ParseFlakeRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Ok(_) => Ok(Self::Pure(s.parse()?)),
            Err(_) => Ok(Self::Impure(s.to_string())),
        }
    }
}

// impl ImpureFlakeRef {
//     pub fn resolve(self) -> Result<FlakeRef, ()> {

//         match self {
//             Self::Pure(flakeref) => return Ok(flakeref),
//             Self::Impure(impure) => {
//                 let path = PathBuf::from(impure);
//                 if let Ok()
//             }
//         }


//         todo!()

//     }


// }





#[derive(Serialize, Deserialize, Display, From, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum FlakeRef {
    FileFile(FileRef<protocol::File>),
    FileHTTP(FileRef<protocol::HTTP>),
    FileHTTPS(FileRef<protocol::HTTPS>),
    TarballFile(TarballRef<protocol::File>),
    TarballHTTP(TarballRef<protocol::HTTP>),
    TarballHTTPS(TarballRef<protocol::HTTPS>),
    Github(GitServiceRef<service::Github>),
    Gitlab(GitServiceRef<service::Gitlab>),
    Path(PathRef),
    GitPath(GitRef<protocol::File>),
    GitSsh(GitRef<protocol::SSH>),
    GitHttps(GitRef<protocol::HTTPS>),
    GitHttp(GitRef<protocol::HTTP>),
    Indirect(IndirectRef),
    // /// https://cs.github.com/NixOS/nix/blob/f225f4307662fe9a57543d0c86c28aa9fddaf0d2/src/libfetchers/tarball.cc#L206
    // Tarball(TarballRef),
}

impl FromStr for FlakeRef {
    type Err = ParseFlakeRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s).unwrap();




        let flake_ref = match url.scheme() {
            protocol::File::scheme() if application::File::required(&url) =>  FileRef::<protocol::File>::from_url(url)?.into(),


            _ if FileRef::<protocol::File>::parses(s) => {
                s.parse::<
            },
            _ if FileRef::<protocol::HTTP>::parses(s) => {
                s.parse::<FileRef<protocol::HTTP>>()?.into()
            },
            _ if FileRef::<protocol::HTTPS>::parses(s) => {
                s.parse::<FileRef<protocol::HTTPS>>()?.into()
            },
            _ if TarballRef::<protocol::File>::parses(s) => {
                s.parse::<TarballRef<protocol::File>>()?.into()
            },
            _ if TarballRef::<protocol::HTTP>::parses(s) => {
                s.parse::<TarballRef<protocol::HTTP>>()?.into()
            },
            _ if TarballRef::<protocol::HTTPS>::parses(s) => {
                s.parse::<TarballRef<protocol::HTTPS>>()?.into()
            },
            _ if GitServiceRef::<service::Github>::parses(s) => {
                s.parse::<GitServiceRef<service::Github>>()?.into()
            },
            _ if GitServiceRef::<service::Gitlab>::parses(s) => {
                s.parse::<GitServiceRef<service::Gitlab>>()?.into()
            },
            _ if PathRef::parses(s) => s.parse::<PathRef>()?.into(),
            _ if GitRef::<protocol::File>::parses(s) => s.parse::<GitRef<protocol::File>>()?.into(),
            _ if GitRef::<protocol::SSH>::parses(s) => s.parse::<GitRef<protocol::SSH>>()?.into(),
            _ if GitRef::<protocol::HTTP>::parses(s) => s.parse::<GitRef<protocol::HTTP>>()?.into(),
            _ if GitRef::<protocol::HTTPS>::parses(s) => {
                s.parse::<GitRef<protocol::HTTPS>>()?.into()
            },
            _ if IndirectRef::parses(s) => s.parse::<IndirectRef>()?.into(),
            _ => Err(ParseFlakeRefError::Invalid)?,
        };
        Ok(flake_ref)
    }
}

#[derive(Debug, Error)]
pub enum ParseFlakeRefError {
    #[error(transparent)]
    File(#[from] file::ParseFileError),
    #[error(transparent)]
    GitService(#[from] git_service::ParseGitServiceError),
    #[error(transparent)]
    Git(#[from] git::ParseGitError),
    #[error(transparent)]
    Indirect(#[from] indirect::ParseIndirectError),
    #[error(transparent)]
    Path(#[from] path::ParsePathRefError),

    #[error("Invalid flakeref")]
    Invalid,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, From, Clone)]
#[serde(try_from = "TimestampDeserialize")]
pub struct Timestamp(
    #[serde(serialize_with = "chrono::serde::ts_seconds::serialize")] chrono::DateTime<chrono::Utc>,
);

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum TimestampDeserialize {
    TsI64(i64),
    TsString(String),
}

impl TryFrom<TimestampDeserialize> for Timestamp {
    type Error = ParseTimeError;

    fn try_from(value: TimestampDeserialize) -> Result<Self, Self::Error> {
        let ts = match value {
            TimestampDeserialize::TsI64(t) => Utc
                .timestamp_opt(t, 0)
                .earliest()
                .ok_or(ParseTimeError::FromInt(t))?,
            // per <https://docs.rs/chrono/0.4.24/chrono/format/strftime/index.html>
            TimestampDeserialize::TsString(s) => NaiveDateTime::parse_from_str(&s, "%s")?
                .and_local_timezone(Utc)
                .earliest()
                .unwrap(),
        };
        Ok(Timestamp(ts))
    }
}

#[derive(Debug, Error)]
pub enum ParseTimeError {
    #[error("Could not parse {0} to UTC date")]
    FromInt(i64),
    #[error(transparent)]
    FromString(#[from] chrono::ParseError),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BoolReprs {
    String(String),
    Bool(bool),
}

impl TryFrom<BoolReprs> for bool {
    type Error = <bool as FromStr>::Err;

    fn try_from(value: BoolReprs) -> Result<Self, Self::Error> {
        match value {
            BoolReprs::String(s) => s.parse::<bool>(),
            BoolReprs::Bool(b) => Ok(b),
        }
    }
}

impl BoolReprs {
    pub fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        BoolReprs::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
pub(super) mod tests {

    pub(super) fn roundtrip_to<T>(input: &str, output: &str)
    where
        T: FromStr + Display,
        <T as FromStr>::Err: Debug + Display,
    {
        let parsed = input
            .parse::<T>()
            .unwrap_or_else(|e| panic!("'{input}' should parse: \n{e}\n{e:#?}"));
        assert_eq!(parsed.to_string(), output);
    }

    pub(super) fn roundtrip<T>(input: &str)
    where
        T: FromStr + Display,
        <T as FromStr>::Err: Debug + Display,
    {
        roundtrip_to::<T>(input, input)
    }

    use std::fmt::Debug;

    use super::*;

    #[test]
    fn test_all_parsing() {
        assert!(matches!(
            dbg!(FlakeRef::from_str("file+file:///somewhere/there")).unwrap(),
            FlakeRef::FileFile(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("file:///somewhere/there")).unwrap(),
            FlakeRef::FileFile(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("file+http://my.de/path/to/file")).unwrap(),
            FlakeRef::FileHTTP(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("http://my.de/path/to/file")).unwrap(),
            FlakeRef::FileHTTP(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("file+https://my.de/path/to/file")).unwrap(),
            FlakeRef::FileHTTPS(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("https://my.de/path/to/file")).unwrap(),
            FlakeRef::FileHTTPS(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("tarball+file:///somewhere/there")).unwrap(),
            FlakeRef::TarballFile(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("file:///somewhere/there.tar.gz")).unwrap(),
            FlakeRef::TarballFile(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("tarball+http://my.de/path/to/file")).unwrap(),
            FlakeRef::TarballHTTP(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("http://my.de/path/to/file.tar.gz")).unwrap(),
            FlakeRef::TarballHTTP(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("tarball+https://my.de/path/to/file")).unwrap(),
            FlakeRef::TarballHTTPS(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("https://my.de/path/to/file.tar.gz")).unwrap(),
            FlakeRef::TarballHTTPS(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("github:flox/runix")).unwrap(),
            FlakeRef::Github(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("gitlab:flox/runix")).unwrap(),
            FlakeRef::Gitlab(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("path:/somewhere/there")).unwrap(),
            FlakeRef::Path(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("git+file:///somewhere/there")).unwrap(),
            FlakeRef::GitPath(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("git+ssh://github.com/flox/runix")).unwrap(),
            FlakeRef::GitSsh(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("git+https://github.com/flox/runix")).unwrap(),
            FlakeRef::GitHttps(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("git+http://github.com/flox/runix")).unwrap(),
            FlakeRef::GitHttp(_)
        ));
        assert!(matches!(
            dbg!(FlakeRef::from_str("flake:nixpkgs")).unwrap(),
            FlakeRef::Indirect(_)
        ));
    }
}
