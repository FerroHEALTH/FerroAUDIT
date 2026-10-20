// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The integration tests of `ihe-atna`, one binary with a module per layer.
//!
//! The modules carry `#[cfg(test)]`, which an integration test always has, so
//! the test-scoped relaxations of `clippy.toml` reach their helpers too.

#[cfg(test)]
#[cfg(any(feature = "atna", feature = "balp"))]
mod atna;
#[cfg(test)]
#[cfg(feature = "balp")]
mod balp;
#[cfg(test)]
mod features;
#[cfg(test)]
#[cfg(any(feature = "atna", feature = "balp"))]
mod timing;
