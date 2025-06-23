// Copyright (C) 2025 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::time::Duration;
use std::{panic, thread};

use crate::burnchains::bitcoin::Error as bitcoin_error;
use crate::burnchains::{Burnchain, Error as burnchain_error};

#[test]
fn join_success() {
    let handle: thread::JoinHandle<Result<u32, burnchain_error>> = thread::spawn(|| Ok(42));

    let result = Burnchain::handle_thread_join(handle, "test");

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn join_returns_error() {
    let handle: thread::JoinHandle<Result<u32, burnchain_error>> = thread::spawn(|| {
        Err(burnchain_error::DownloadError(
            bitcoin_error::ConnectionError,
        ))
    });

    let result = Burnchain::handle_thread_join(handle, "test");

    assert!(result.is_err());
    match result {
        Err(burnchain_error::DownloadError(_)) => {}
        _ => panic!("Expected DownloadError"),
    }
}

#[test]
fn join_panics() {
    let handle: thread::JoinHandle<Result<u32, burnchain_error>> = thread::spawn(|| {
        panic!("boom");
        #[allow(unreachable_code)]
        Ok(42)
    });

    let result = Burnchain::handle_thread_join(handle, "test");

    assert!(result.is_err());
    match result {
        Err(burnchain_error::ThreadChannelError) => {}
        _ => panic!("Expected ThreadChannelError"),
    }
}

#[test]
fn join_with_delay() {
    let handle: thread::JoinHandle<Result<u32, burnchain_error>> = thread::spawn(|| {
        thread::sleep(Duration::from_millis(100));
        Ok(42)
    });

    let result = Burnchain::handle_thread_join(handle, "test");

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}
