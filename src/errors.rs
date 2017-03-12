// This file is part of the Rust library `subparse`.
//
// Copyright (C) 2017 kaegi
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

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
