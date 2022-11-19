//! This module contains data types that can be used in the model when special input/output
//! (ser/de) behavior is desired.  For example, the ValidBase64 type can be used for a model field
//! when we don't even want to accept an API call with invalid base64 data.

// The pattern in this module is to make a struct and implement TryFrom<&str> with code that does
// necessary checks and returns the struct.  Other traits that treat the struct like a string can
// be implemented for you with the string_impls_for macro.

pub mod error {
    use regex::Regex;
    use scalar::ValidationError;
    use snafu::Snafu;

    // x509_parser::pem::Pem::parse_x509 returns an Err<X509Error>, which is a bit
    // verbose. Declaring a type to simplify it.
    type PEMToX509ParseError = x509_parser::nom::Err<x509_parser::error::X509Error>;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Can't create SingleLineString containing line terminator"))]
        StringContainsLineTerminator,

        #[snafu(display("Invalid base64 input: {}", source))]
        InvalidBase64 { source: base64::DecodeError },

        #[snafu(display(
            "Identifiers may only contain ASCII alphanumerics plus hyphens, received '{}'",
            input
        ))]
        InvalidIdentifier { input: String },

        #[snafu(display(
            "Kernel boot config keywords may only contain ASCII alphanumerics plus hyphens and underscores, received '{}'",
            input
        ))]
        InvalidBootconfigKey { input: String },

        #[snafu(display(
            "Kernel boot config values may only contain ASCII printable characters, received '{}'",
            input
        ))]
        InvalidBootconfigValue { input: String },

        #[snafu(display(
            "Kernel module keys may only contain ASCII alphanumerics plus hyphens and underscores, received '{}'",
            input
        ))]
        InvalidKmodKey { input: String },

        #[snafu(display("Given invalid URL '{}'", input))]
        InvalidUrl { input: String },

        #[snafu(display("Invalid version string '{}'", input))]
        InvalidVersion { input: String },

        #[snafu(display("{} must match '{}', given: {}", thing, pattern, input))]
        Pattern {
            thing: String,
            pattern: Regex,
            input: String,
        },

        // Some regexes are too big to usefully display in an error.
        #[snafu(display("{} given invalid input: {}", thing, input))]
        BigPattern { thing: String, input: String },

        #[snafu(display("Invalid Kubernetes cloud provider '{}'", input))]
        InvalidCloudProvider { input: String },

        #[snafu(display("Invalid Kubernetes authentication mode '{}'", input))]
        InvalidAuthenticationMode { input: String },

        #[snafu(display("Invalid bootstrap container mode '{}'", input))]
        InvalidBootstrapContainerMode { input: String },

        #[snafu(display("Given invalid cluster name '{}': {}", name, msg))]
        InvalidClusterName { name: String, msg: String },

        #[snafu(display("Invalid domain name '{}': {}", input, msg))]
        InvalidDomainName { input: String, msg: String },

        #[snafu(display("Invalid hostname '{}': {}", input, msg))]
        InvalidLinuxHostname { input: String, msg: String },

        #[snafu(display("Invalid Linux lockdown mode '{}'", input))]
        InvalidLockdown { input: String },

        #[snafu(display("Invalid sysctl key '{}': {}", input, msg))]
        InvalidSysctlKey { input: String, msg: String },

        #[snafu(display("Invalid input for field {}: {}", field, source))]
        InvalidPlainValue {
            field: String,
            source: serde_plain::Error,
        },

        #[snafu(display("Invalid Kubernetes threshold percentage value '{}'", input))]
        InvalidThresholdPercentage { input: String },

        #[snafu(display("Invalid percentage value '{}'", input))]
        InvalidPercentage {
            input: String,
            source: std::num::ParseFloatError,
        },

        #[snafu(display("Invalid Cpu Manager policy '{}'", input))]
        InvalidCpuManagerPolicy {
            input: String,
            source: serde_plain::Error,
        },

        #[snafu(display("Invalid Kubernetes duration value '{}'", input))]
        InvalidKubernetesDurationValue { input: String },

        #[snafu(display("Invalid x509 certificate: {}", source))]
        InvalidX509Certificate { source: PEMToX509ParseError },

        #[snafu(display("Invalid PEM object: {}", source))]
        InvalidPEM {
            source: x509_parser::error::PEMError,
        },

        #[snafu(display("No valid certificate found in bundle"))]
        NoCertificatesFound {},

        #[snafu(display("Invalid topology manager scope '{}'", input))]
        InvalidTopologyManagerScope {
            input: String,
            source: serde_plain::Error,
        },

        #[snafu(display("Invalid topology manager policy '{}'", input))]
        InvalidTopologyManagerPolicy {
            input: String,
            source: serde_plain::Error,
        },

        #[snafu(display("Invalid imageGCHighThresholdPercent '{}': {}", input, msg))]
        InvalidImageGCHighThresholdPercent { input: String, msg: String },

        #[snafu(display("Invalid imageGCLowThresholdPercent '{}': {}", input, msg))]
        InvalidImageGCLowThresholdPercent { input: String, msg: String },

        #[snafu(display("Invalid ECS duration value '{}'", input))]
        InvalidECSDurationValue { input: String },

        #[snafu(display("Could not parse '{}' as an integer", input))]
        ParseInt {
            input: String,
            source: std::num::ParseIntError,
        },
    }

    /// Creates a `ValidationError` with a consistent message for strings with regex validations
    /// where the regex is too big to display to the user.
    pub(crate) fn big_pattern_error<S1, S2>(thing: S1, input: S2) -> ValidationError
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        ValidationError::new(format!(
            "{} given invalid input: {}",
            thing.as_ref(),
            input.as_ref()
        ))
    }
}

/// Helper macro for implementing the common string-like traits for a modeled type.
/// Pass the name of the type, and the name of the type in quotes (to be used in string error
/// messages, etc.).
macro_rules! string_impls_for {
    ($for:ident, $for_str:expr) => {
        impl TryFrom<String> for $for {
            type Error = $crate::modeled_types::error::Error;

            fn try_from(input: String) -> Result<Self, Self::Error> {
                Self::try_from(input.as_ref())
            }
        }

        impl<'de> Deserialize<'de> for $for {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let original = String::deserialize(deserializer)?;
                Self::try_from(original).map_err(|e| {
                    D::Error::custom(format!("Unable to deserialize into {}: {}", $for_str, e))
                })
            }
        }

        /// We want to serialize the original string back out, not our structure, which is just there to
        /// force validation.
        impl Serialize for $for {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.inner)
            }
        }

        impl Deref for $for {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl Borrow<String> for $for {
            fn borrow(&self) -> &String {
                &self.inner
            }
        }

        impl Borrow<str> for $for {
            fn borrow(&self) -> &str {
                &self.inner
            }
        }

        impl AsRef<str> for $for {
            fn as_ref(&self) -> &str {
                &self.inner
            }
        }

        impl fmt::Display for $for {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.inner)
            }
        }

        impl From<$for> for String {
            fn from(x: $for) -> Self {
                x.inner
            }
        }

        impl PartialEq<str> for $for {
            fn eq(&self, other: &str) -> bool {
                &self.inner == other
            }
        }

        impl PartialEq<String> for $for {
            fn eq(&self, other: &String) -> bool {
                &self.inner == other
            }
        }

        impl PartialEq<&str> for $for {
            fn eq(&self, other: &&str) -> bool {
                &self.inner == other
            }
        }
    };
}

/// This is similar to the `Snafu` `ensure` macro that we are familiar with, but it works with our
/// own `ValidationError` instead of a `Snafu` error enum.
macro_rules! require {
    ($condition:expr, $err:expr) => {
        if !($condition) {
            return Err($err);
        }
    };
}

// Must be after macro definition
mod ecs;
mod kubernetes;
mod shared;

pub use ecs::*;
pub use kubernetes::*;
pub use shared::*;
