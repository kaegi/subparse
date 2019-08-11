// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::SubtitleFormat;
use failure::Backtrace;
use failure::Context;
use failure::Fail;
use std::fmt;

pub use crate::formats::idx::errors as idx_errors;
pub use crate::formats::microdvd::errors as mdvd_errors;

pub use crate::formats::srt::errors as srt_errors;
pub use crate::formats::ssa::errors as ssa_errors;
pub use crate::formats::vobsub::errors as vob_errors;

/// A result type that can be used wide for error handling.
pub type Result<T> = std::result::Result<T, Error>;

/// The error structure which containes, a backtrace, causes and the error kind enum variant.
#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
/// Error kind for a crate-wide error.
pub enum ErrorKind {
    /// Parsing error
    ParsingError,

    /// The file format is not supported by this library.
    UnknownFileFormat,

    /// The file format is not supported by this library.
    DecodingError,

    /// The attempted operation does not work on binary subtitle formats.
    TextFormatOnly,

    /// The attempted operation does not work on this format (not supported in this version of this library).
    UpdatingEntriesNotSupported {
        /// The format for which updating the subtitle entries is not supported.
        format: SubtitleFormat,
    },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::ParsingError => write!(f, "parsing the subtitle data failed"),
            ErrorKind::UnknownFileFormat => write!(
                f,
                "unknown file format, only SubRip (.srt), SubStationAlpha (.ssa/.ass) and VobSub (.idx and .sub) are supported at the moment"
            ),
            ErrorKind::DecodingError => write!(f, "error while decoding subtitle from bytes to string (wrong charset encoding?)"),
            ErrorKind::TextFormatOnly => write!(f, "operation does not work on binary subtitle formats (only text formats)"),
            ErrorKind::UpdatingEntriesNotSupported { format } => write!(
                f,
                "updating subtitles is not implemented or supported by the `subparse` library for this format: {}",
                format.get_name()
            ),
        }
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl Error {
    /// Returns the actual error kind for this error.
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { inner: Context::new(kind) }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner: inner }
    }
}

// see https://docs.rs/error-chain/0.8.1/error_chain/
/*#[cfg_attr(rustfmt, rustfmt_skip)]
error_chain! {
    foreign_links {
        FromUtf8Error(::std::string::FromUtf8Error)
        /// Converting byte-stream to string failed.
        ;
    }


    links {
        SsaError(ssa_errors::Error, ssa_errors::ErrorKind)
        /// Parsing a `.ssa`/`.ass` file failed.
        ;

        IdxError(idx_errors::Error, idx_errors::ErrorKind)
        /// Parsing a `.idx` file failed.
        ;

        SrtError(srt_errors::Error, srt_errors::ErrorKind)
        /// Parsing a `.srt` file failed.
        ;

        VobError(vob_errors::Error, vob_errors::ErrorKind)
        /// Parsing a `.sub` (VobSub) file failed.
        ;

        MdvdError(mdvd_errors::Error, mdvd_errors::ErrorKind)
        /// Parsing a `.sub` (MicroDVD) file failed.
        ;
    }

    errors {
        /// The file format is not supported by this library.
        UnknownFileFormat {
            description("unknown file format, only SubRip (.srt), SubStationAlpha (.ssa/.ass) and VobSub (.idx and .sub) are supported at the moment")
        }

        /// The file format is not supported by this library.
        DecodingError {
            description("error while decoding subtitle from bytes to string (wrong charset encoding?)")
        }

        /// The attempted operation does not work on binary subtitle formats.
        TextFormatOnly {
            description("operation does not work on binary subtitle formats (only text formats)")
        }

        /// The attempted operation does not work on this format (not supported in this version of this library).
        UpdatingEntriesNotSupported(format: SubtitleFormat) {
            description("updating subtitles is not implemented or supported by the `subparse` library for this format")
            display("updating subtitles is not implemented or supported by the `subparse` library for this format: {}", format.get_name())
        }
    }
}*/

#[macro_use]
/// Creates the `Error`-context type for an ErrorKind and associated conversion methods.
macro_rules! define_error {
    ($error:ident, $kind:ident) => {
        use failure::Fail;
        use failure::{Backtrace, Context};
        use std::fmt;

        /// The error structure which containes, a backtrace, causes and the error kind enum variant.
        #[derive(Debug)]
        pub struct $error {
            inner: Context<$kind>,
        }

        impl Fail for $error {
            fn cause(&self) -> Option<&Fail> {
                self.inner.cause()
            }

            fn backtrace(&self) -> Option<&Backtrace> {
                self.inner.backtrace()
            }
        }

        impl fmt::Display for $error {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(&self.inner, f)
            }
        }

        impl $error {
            /// Get inner error enum variant.
            pub fn kind(&self) -> &$kind {
                self.inner.get_context()
            }
        }

        impl From<$kind> for $error {
            fn from(kind: $kind) -> $error {
                $error { inner: Context::new(kind) }
            }
        }

        impl From<Context<$kind>> for $error {
            fn from(inner: Context<$kind>) -> $error {
                $error { inner: inner }
            }
        }
    };
}
