//! # madhouse-rs
//!
//! A framework for model-based stateful testing.
//!
//! This library provides infrastructure for writing property-based tests
//! that exercise stateful systems through sequences of commands. It supports
//! both deterministic and random testing approaches.
//!
//! ## Overview
//!
//! Stateful systems often have complex behaviors:
//! - Many hardcoded test sequences are needed.
//! - Timing-dependent behavior is hard to test systematically.
//! - Manual test case construction is slow.
//! - Properties span multiple operations.
//!
//! This framework implements state machine testing:
//!
//! ```text
//!                    +-------+
//!                    | State |
//!                    +-------+
//!                        ^
//!                        |
//!   +---------+     +----+----+     +-----------+
//!   | Command | --> | check() | --> |  apply()  |
//!   +---------+     +---------+     | [asserts] |
//!                                   +-----------+
//!        ^                                |
//!        |                                v
//!   +----------+                      +--------+
//!   | Strategy |                      | State' |
//!   +----------+                      +--------+
//! ```

use proptest::prelude::Strategy;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;
use std::time::Instant;

/// The State trait represents the system state being tested.
/// Implement this trait for your specific system state.
pub trait State: Debug {}

/// The TestContext trait represents the test configuration.
/// Implement this trait for your specific test context.
pub trait TestContext: Debug + Clone {}

/// Trait for commands in the stateful testing framework.
/// Each command represents an action that can be performed in the system.
/// Commands are responsible for:
/// - Checking if they can be applied to the current state.
/// - Applying themselves to modify the state.
/// - Providing a descriptive label.
/// - Building a strategy for generating instances of the command.
pub trait Command<S: State, C: TestContext> {
    /// Checks if the command can be applied to the current state.
    /// Returns true if the command can be applied, false otherwise.
    ///
    /// # Arguments
    /// * `state` - The current state to check against.
    fn check(&self, state: &S) -> bool;

    /// Applies the command to the state, modifying it.
    /// This method should only be called if `check` returns true.
    /// It can include assertions to verify correctness.
    ///
    /// # Arguments
    /// * `state` - The state to modify.
    fn apply(&self, state: &mut S);

    /// Returns a human-readable label for the command.
    /// Used for debugging and test output.
    fn label(&self) -> String;

    /// Builds a proptest strategy for generating instances of this command.
    ///
    /// # Arguments
    /// * `ctx` - Test context used to parameterize command generation.
    fn build(ctx: Arc<C>) -> impl Strategy<Value = CommandWrapper<S, C>>
    where
        Self: Sized;
}

/// Wrapper for command trait objects.
/// This wrapper allows commands to be stored in collections and
/// passed between functions while preserving their concrete type.
/// It provides a convenient way to implement Debug for dynamic Commands.
pub struct CommandWrapper<S: State, C: TestContext> {
    /// The wrapped command trait object.
    pub command: Arc<dyn Command<S, C>>,
}

impl<S: State, C: TestContext> CommandWrapper<S, C> {
    /// Creates a new command wrapper for the given command.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to wrap.
    pub fn new<Cmd: Command<S, C> + 'static>(cmd: Cmd) -> Self {
        Self {
            command: Arc::new(cmd),
        }
    }
}

impl<S: State, C: TestContext> Clone for CommandWrapper<S, C> {
    fn clone(&self) -> Self {
        Self {
            command: Arc::clone(&self.command),
        }
    }
}

impl<S: State, C: TestContext> Debug for CommandWrapper<S, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.command.label())
    }
}

/// Creates a strategy that always returns a Vec containing values from all the
/// provided strategies, in the exact order they were passed.
///
/// This is similar to `prop_oneof` but instead of randomly picking strategies,
/// it includes values from all strategies in a Vec.
#[macro_export]
macro_rules! prop_allof {
    ($strat:expr $(,)?) => {
        $strat.prop_map(|val| vec![val])
    };

    ($first:expr, $($rest:expr),+ $(,)?) => {
        {
            let first_strat = $first.prop_map(|val| vec![val]);
            let rest_strat = prop_allof!($($rest),+);

            (first_strat, rest_strat).prop_map(|(mut first_vec, rest_vec)| {
                first_vec.extend(rest_vec);
                first_vec
            })
        }
    };
}

/// Executes a sequence of commands and returns those that were executed.
///
/// This function:
/// 1. Filters commands based on their `check` method.
/// 2. Applies each valid command to the state.
/// 3. Measures execution time for each command.
/// 4. Prints a colored summary of selected and executed commands.
///
/// # Arguments
/// * `commands` - Slice of commands to potentially execute.
/// * `state` - Mutable state that commands will check against and modify.
///
/// # Returns
/// A vector of references to commands that were actually executed.
pub fn execute_commands<'a, S: State, C: TestContext>(
    commands: &'a [CommandWrapper<S, C>],
    state: &mut S,
) -> Vec<&'a CommandWrapper<S, C>> {
    let mut executed = Vec::with_capacity(commands.len());
    let mut execution_times = Vec::with_capacity(commands.len());

    // ANSI color codes.
    let yellow = "\x1b[33m";
    let green = "\x1b[32m";
    let reset = "\x1b[0m";

    for cmd in commands {
        if cmd.command.check(state) {
            let start = Instant::now();
            cmd.command.apply(state);
            let duration = start.elapsed();
            executed.push(cmd);
            execution_times.push(duration);
        }
    }

    println!("Selected:");
    for (i, cmd) in commands.iter().enumerate() {
        println!("{:02}. {}{}{}", i + 1, yellow, cmd.command.label(), reset);
    }

    println!("Executed:");
    for (i, (cmd, time)) in executed.iter().zip(execution_times.iter()).enumerate() {
        println!(
            "{:02}. {}{}{} ({:.2?})",
            i + 1,
            green,
            cmd.command.label(),
            reset,
            time
        );
    }

    executed
}

/// Macro for running stateful tests.
///
/// By default, commands are executed deterministically in the order
/// they are passed. If the `MADHOUSE=1` environment variable is set
/// commands are executed randomly.
///
/// This macro configures proptest to:
/// - Run a single test case (cases = 1).
/// - Skip shrinking (max_shrink_iters = 0).
/// - Use either random or deterministic command generation.
///
/// # Arguments
///
/// * `test_context` - The test context to use for creating commands.
/// * `command1, command2, ...` - The command types to test.
#[macro_export]
macro_rules! scenario {
    ($test_context:expr, $($cmd_type:ident),+ $(,)?) => {
        {
            let test_context = $test_context.clone();
            let config = proptest::test_runner::Config {
                cases: 1,
                max_shrink_iters: 0,
                ..Default::default()
            };

            // Use MADHOUSE env var to determine test mode.
            let use_madhouse = ::std::env::var("MADHOUSE") == Ok("1".into());

            if use_madhouse {
                proptest::proptest!(config, |(commands in proptest::collection::vec(
                    proptest::prop_oneof![
                        $($cmd_type::build(test_context.clone())),+
                    ],
                    1..16,
                ))| {
                    println!("\n=== New Test Run (MADHOUSE mode) ===\n");
                    let mut state = <_ as ::std::default::Default>::default();
                    $crate::tests::signer::v0::execute_commands(&commands, &mut state);
                });
            } else {
                proptest::proptest!(config, |(commands in $crate::prop_allof![
                    $($cmd_type::build(test_context.clone())),+
                ])| {
                    println!("\n=== New Test Run (deterministic mode) ===\n");
                    let mut state = <_ as ::std::default::Default>::default();
                    $crate::tests::signer::v0::execute_commands(&commands, &mut state);
                });
            }
        }
    };
}
