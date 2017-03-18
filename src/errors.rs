// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


pub use formats::srt::errors as srt_errors;
pub use formats::ssa::errors as ssa_errors;
pub use formats::idx::errors as idx_errors;

// see https://docs.rs/error-chain/0.8.1/error_chain/
#[cfg_attr(rustfmt, rustfmt_skip)]
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
    }

    errors {
        /// The file format is not supported by this library.
        UnknownFileFormat {
            description("unknown file format, only SubRip (.srt), SubStationAlpha (.ssa/.ass) and VobSub (.idx) are supported at the moment")
        }
    }
}
