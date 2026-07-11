//! 1BitShit CPU binary entrypoint.
//!
//! The complete CLI implementation remains in `main.rs`. This entrypoint keeps
//! that implementation intact while replacing the obsolete Ghost terminology
//! at the presentation boundary.

macro_rules! eprintln {
    (
        "  {} [Ghost Execution Detected] You are running a local binary at {:?}",
        $icon:expr,
        $path:expr
    ) => {
        ::std::eprintln!(
            "  {} [1BitShit Development Runtime] Running local binary at {:?}",
            $icon,
            $path
        )
    };
    ($($argument:tt)*) => {
        ::std::eprintln!($($argument)*)
    };
}

include!("main.rs");
