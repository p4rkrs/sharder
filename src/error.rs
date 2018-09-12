use redis_async::error::Error as RedisError;
use serde_json::Error as JsonError;
use serenity::Error as SerenityError;
use std::{
    error::Error as StdError,
    env::VarError,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IoError,
    net::AddrParseError,
    num::ParseIntError,
    result::Result as StdResult,
};
use tungstenite::Error as TungsteniteError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    AddrParse(AddrParseError),
    Io(IoError),
    Json(JsonError),
    ParseInt(ParseIntError),
    Redis(RedisError),
    Serenity(SerenityError),
    Tungstenite(TungsteniteError),
    Var(VarError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.write_str(self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::AddrParse(why) => why.description(),
            Error::Io(why) => why.description(),
            Error::Json(why) => why.description(),
            Error::ParseInt(why) => why.description(),
            Error::Redis(why) => why.description(),
            Error::Serenity(why) => why.description(),
            Error::Tungstenite(why) => why.description(),
            Error::Var(why) => why.description(),
        }
    }
}

impl From<AddrParseError> for Error {
    fn from(e: AddrParseError) -> Self {
        Error::AddrParse(e)
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

impl From<JsonError> for Error {
    fn from(e: JsonError) -> Self {
        Error::Json(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Error::ParseInt(e)
    }
}

impl From<RedisError> for Error {
    fn from(e: RedisError) -> Self {
        Error::Redis(e)
    }
}

impl From<SerenityError> for Error {
    fn from(e: SerenityError) -> Self {
        Error::Serenity(e)
    }
}

impl From<TungsteniteError> for Error {
    fn from(e: TungsteniteError) -> Self {
        Error::Tungstenite(e)
    }
}

impl From<VarError> for Error {
    fn from(e: VarError) -> Self {
        Error::Var(e)
    }
}
