// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std;
use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use package::PackageTarget;

use serde::Deserialize;
use toml;

/// Description for errors which can occur when working with a configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// Expected a valid array of values for configuration field value.
    BadArray(&'static str),
    /// Expected a valid network address for configuration field value.
    BadIpAddr(&'static str),
    /// Expected a valid SocketAddr address pair for configuration field value.
    BadSocketAddr(&'static str),
    /// Expected a string for configuration field value.
    BadString(&'static str),
    DecodeError(toml::DecodeError),
    /// Error reading raw contents of configuration file.
    IO(io::Error),
    /// Expected a valid target string for configuration field value.
    InvalidTargetString(&'static str),
    /// Parsing error while reading a configuration file.
    ParseError(Vec<toml::ParserError>),
}

impl error::Error for ConfigError {
    fn description(&self) -> &str {
        match *self {
            ConfigError::BadArray(_) => {
                "Invalid array of values encountered while parsing a configuration file"
            }
            ConfigError::BadIpAddr(_) => {
                "Invalid network address encountered while parsing a configuration file"
            }
            ConfigError::BadSocketAddr(_) => {
                "Invalid network address pair encountered while parsing a configuration file"
            }
            ConfigError::BadString(_) => {
                "Invalid string value encountered while parsing a configuration file"
            }
            ConfigError::DecodeError(_) => "Unable to decode raw contents into configuration.",
            ConfigError::IO(_) => "Unable to read the raw contents of a configuration file.",
            ConfigError::InvalidTargetString(_) => {
                "Invalid target string value encountered while parsing a configuration file"
            }
            ConfigError::ParseError(_) => "Error parsing contents of configuration file.",
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            ConfigError::BadArray(ref e) => {
                format!("Invalid array of values in config, field={}", e)
            }
            ConfigError::BadIpAddr(ref e) => {
                format!("Invalid address in config, field={}. (example: \"127.0.0.0\")",
                        e)
            }
            ConfigError::BadSocketAddr(ref e) => {
                format!("Invalid network address pair in config, field={}. (example: \
                         \"127.0.0.0:8080\")",
                        e)
            }
            ConfigError::BadString(ref e) => {
                format!("Invalid string value in config, field={}.", e)
            }
            ConfigError::DecodeError(ref e) => format!("Unable to decode configuration, {}", e),
            ConfigError::IO(ref e) => format!("Unable to read contents of configuration, {}", e),
            ConfigError::InvalidTargetString(ref f) => {
                format!("Invalid target string value in config, field={}.", f)
            }
            ConfigError::ParseError(ref errors) => {
                let mut msg = String::new();
                for err in errors {
                    msg.push_str(&format!("\terror: {}\n", err.desc));
                }
                format!("Syntax error in configuration:\n{}", msg)
            }
        };
        write!(f, "{}", msg)
    }
}

pub trait ConfigFile: Sized + Deserialize + Default {
    type Error: std::error::Error + From<ConfigError>;

    fn from_file<T: AsRef<Path>>(filepath: T) -> Result<Self, Self::Error> {
        let mut file = match File::open(filepath.as_ref()) {
            Ok(f) => f,
            Err(e) => return Err(Self::Error::from(ConfigError::IO(e))),
        };
        let mut raw = String::new();
        match file.read_to_string(&mut raw) {
            Ok(_) => Self::from_str(&raw),
            Err(e) => Err(Self::Error::from(ConfigError::IO(e))),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_str(&String::from_utf8_lossy(bytes))
    }

    fn from_str(raw: &str) -> Result<Self, Self::Error> {
        let toml = try!(raw.parse::<toml::Value>()
            .map_err(|e| Self::Error::from(ConfigError::ParseError(e))));
        Self::from_toml(toml)
    }

    fn from_toml(toml: toml::Value) -> Result<Self, Self::Error> {
        Deserialize::deserialize(&mut toml::Decoder::new(toml))
            .map_err(|e| Self::Error::from(ConfigError::DecodeError(e)))
    }
}

pub trait ParseInto<T> {
    fn parse_into(&self, field: &'static str, out: &mut T) -> Result<bool, ConfigError>;
}

impl ParseInto<Vec<SocketAddr>> for toml::Value {
    fn parse_into(&self,
                  field: &'static str,
                  out: &mut Vec<SocketAddr>)
                  -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(slice) = val.as_slice() {
                let mut buf = vec![];
                for entry in slice.iter() {
                    if let Some(v) = entry.as_str() {
                        match SocketAddr::from_str(v) {
                            Ok(addr) => buf.push(addr),
                            Err(_) => return Err(ConfigError::BadSocketAddr(field)),
                        }
                    } else {
                        return Err(ConfigError::BadSocketAddr(field));
                    }
                }
                *out = buf;
                Ok(true)
            } else {
                // error, expected array
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<SocketAddr> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut SocketAddr) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_str() {
                match SocketAddr::from_str(v) {
                    Ok(addr) => {
                        *out = addr;
                        Ok(true)
                    }
                    Err(_) => Err(ConfigError::BadSocketAddr(field)),
                }
            } else {
                Err(ConfigError::BadSocketAddr(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<IpAddr> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut IpAddr) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_str() {
                match IpAddr::from_str(v) {
                    Ok(addr) => {
                        *out = addr;
                        Ok(true)
                    }
                    Err(_) => Err(ConfigError::BadIpAddr(field)),
                }
            } else {
                Err(ConfigError::BadIpAddr(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<String> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut String) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_str() {
                *out = v.to_string();
                Ok(true)
            } else {
                Err(ConfigError::BadString(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<Option<String>> for toml::Value {
    fn parse_into(&self,
                  field: &'static str,
                  out: &mut Option<String>)
                  -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_str() {
                *out = Some(v.to_string());
                Ok(true)
            } else {
                Err(ConfigError::BadString(field))
            }
        } else {
            *out = None;
            Ok(true)
        }
    }
}

impl ParseInto<bool> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut bool) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_bool() {
                *out = v as bool;
                Ok(true)
            } else {
                Err(ConfigError::BadString(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<usize> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut usize) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_integer() {
                *out = v as usize;
                Ok(true)
            } else {
                Err(ConfigError::BadString(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<u16> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut u16) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_integer() {
                *out = v as u16;
                Ok(true)
            } else {
                Err(ConfigError::BadString(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<u32> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut u32) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_integer() {
                *out = v as u32;
                Ok(true)
            } else {
                Err(ConfigError::BadString(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<u64> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut u64) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_integer() {
                *out = v as u64;
                Ok(true)
            } else {
                Err(ConfigError::BadString(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<Vec<u16>> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut Vec<u16>) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_slice() {
                let mut buf = vec![];
                for int in v.iter() {
                    if let Some(i) = int.as_integer() {
                        buf.push(i as u16);
                    } else {
                        return Err(ConfigError::BadArray(field));
                    }
                }
                *out = buf;
                Ok(true)
            } else {
                Err(ConfigError::BadArray(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<Vec<u32>> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut Vec<u32>) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_slice() {
                let mut buf = vec![];
                for int in v.iter() {
                    if let Some(i) = int.as_integer() {
                        buf.push(i as u32);
                    } else {
                        return Err(ConfigError::BadArray(field));
                    }
                }
                *out = buf;
                Ok(true)
            } else {
                Err(ConfigError::BadArray(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<Vec<u64>> for toml::Value {
    fn parse_into(&self, field: &'static str, out: &mut Vec<u64>) -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_slice() {
                let mut buf = vec![];
                for int in v.iter() {
                    if let Some(i) = int.as_integer() {
                        buf.push(i as u64);
                    } else {
                        return Err(ConfigError::BadArray(field));
                    }
                }
                *out = buf;
                Ok(true)
            } else {
                Err(ConfigError::BadArray(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<BTreeMap<String, String>> for toml::Value {
    fn parse_into(&self,
                  field: &'static str,
                  out: &mut BTreeMap<String, String>)
                  -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            let buf: BTreeMap<String, String> = val.as_table()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.to_string(), v.as_str().unwrap().to_string()))
                .collect();
            *out = buf;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<Vec<BTreeMap<String, String>>> for toml::Value {
    fn parse_into(&self,
                  field: &'static str,
                  out: &mut Vec<BTreeMap<String, String>>)
                  -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_slice() {
                let mut buf = vec![];
                for m in v.iter() {
                    let map: BTreeMap<String, String> = m.as_table()
                        .unwrap()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.as_str().unwrap().to_string()))
                        .collect();
                    buf.push(map);
                }
                *out = buf;
                Ok(true)
            } else {
                Err(ConfigError::BadArray(field))
            }
        } else {
            Ok(false)
        }
    }
}

impl ParseInto<PackageTarget> for toml::Value {
    fn parse_into(&self,
                  field: &'static str,
                  out: &mut PackageTarget)
                  -> Result<bool, ConfigError> {
        if let Some(val) = self.lookup(field) {
            if let Some(v) = val.as_str() {
                *out = PackageTarget::from_str(v).unwrap();
                Ok(true)
            } else {
                Err(ConfigError::InvalidTargetString(field))
            }
        } else {
            Ok(true)
        }
    }
}
