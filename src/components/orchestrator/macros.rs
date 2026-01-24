use std::time::Duration;

use common_game::logging::Channel;

pub const LOG_FN_CALL_CHNL: Channel = Channel::Debug;
///The events this level should be used for are:
/// function call (and finish if it is relevant) and parameters.
/// every interaction between actors that is not covered in Info
/// such as every other message
pub const LOG_FN_INT_OPERATIONS: Channel = Channel::Trace;
/// every operation that it useful to log inside a function
/// such as to log changes made to variables
pub const LOG_ACTORS_ACTIVITY: Channel = Channel::Info;
///The events this level should be used for are:
///Planet creation,destruction,start,stop
///Explorer movement,death,start/stop

//LOG macros
//in order to reduce code duplication

/// Creates a structured metadata map for log events.
///
/// This macro abstracts the boilerplate of manual BTreeMap instantiation
/// and string conversion. It ensures that all log metadata follows a
/// consistent key-value format.
///
/// * `$key => $val` - pairs of data to be stored as the event's payload
#[macro_export] //make this macro visible outside
macro_rules! payload {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut p = std::collections::BTreeMap::new();
        $(
            p.insert($key.to_string(), $val.to_string());
        )*
        p
    }};
}
/// Generates a standardized payload for system warnings and errors.
///
/// This macro captures the context of an error alongside the function
/// name and live variables.
///
/// * `$warn` - the high-level warning category or message
/// * `$err` - the specific error or exception returned by the system
/// * `$func` - the name of the function where the failure occurred
/// * `$param` - local variables to be stringified for debugging context
#[macro_export]
macro_rules! warning_payload {
    ($warn:expr, $err:expr, $func:expr $(,$param:ident )*$(; $($key:expr => $val:expr),*)?) => {{
        let mut p = std::collections::BTreeMap::new();

        p.insert("Warning".to_string(), $warn.to_string());
        p.insert("returned error".to_string(), $err.to_string());
        p.insert("fn".to_string(), $func.to_string());

        // adds every argument
        $(
            p.insert(
                stringify!($param).to_string(),
                format!("{:?}", $param)
            );
        )*
        // generic key-value
        $($(
            p.insert($key.to_string(), $val.to_string());
        )*)?

        p
    }};
}
/// Logs Orchestrator functions internal actions.
///
/// This macro specializes self-directed logging for the Orchestrator (ID 0).
///
/// * `$key => $val` - internal state data to be recorded
/// * `$msg` - shorthand for a simple action description
#[macro_export]
macro_rules! log_orch_internal {
    ({ $($key:expr => $val:expr),* $(,)? }) => {{
        use common_game::logging::{LogEvent, Participant, ActorType, EventType};

        LogEvent::self_directed(
            Participant::new(ActorType::Orchestrator, 0u32),
            EventType::InternalOrchestratorAction,
            $crate::components::orchestrator::macros::LOG_FN_INT_OPERATIONS,
            $crate::payload!( $($key => $val),* )
        ).emit();
    }};
    // for easily write one element in the payload
    ($msg:expr) => {
        $crate::log_orch_internal!({ "action" => $msg });
    };
}
/// Records Orchestrator function execution and input arguments.
///
/// This macro automatically captures the function name and its parameters.
/// It helps to verify if functions are called with the expected values.
///
/// * `$fn_name` - the name of the function being executed
/// * `$param` - identifiers of the variables to be captured as arguments
/// * `$key => $val` - optional extra metadata for the call
#[macro_export]
macro_rules! log_orch_fn {
    (
        $fn_name:expr
        // section that accept the function arguments
        $(, $param:ident)* // section for generic key-value elements (introduced by ';')
        $(; $($key:expr => $val:expr),*)?
        $(,)?
    ) => {{
        use common_game::logging::{LogEvent, Participant, ActorType, EventType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        // adding function arguments
        $(
            p.insert(
                stringify!($param).to_string(),
                format!("{:?}", $param)
            );
        )*

        // generic key-value
        $($(
            p.insert($key.to_string(), $val.to_string());
        )*)?

        LogEvent::self_directed(
            Participant::new(ActorType::Orchestrator, 0u32),
            EventType::InternalOrchestratorAction,
            $crate::components::orchestrator::macros::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};
}
/// Traces communication and message flow between different actors.
///
/// This macro unifies the logging of messages to visualize the
/// interaction flow between game entities.
///
/// * `$from_actor / $from_id` - the source of the message
/// * `$to_actor / $to_id` - the intended recipient
/// * `$event_type` - the nature of the event
/// * `$message` - the content or identifier of the message sent/received
#[macro_export]
macro_rules! log_message {
    (
        $from_actor:expr, $from_id:expr,
        $to_actor:expr, $to_id:expr,
        $event_type:expr,
        $message:expr
        $(, $param:ident)*
        $(; $($key:expr => $val:expr),*)?
        $(,)?
    ) => {{
        use common_game::logging::{LogEvent, Participant};

        let mut p = std::collections::BTreeMap::new();
        p.insert("message".to_string(), $message.to_string());

        // adding parameters
        $(
            p.insert(
                stringify!($param).to_string(),
                format!("{:?}", $param)
            );
        )*

        // generic key-value pairs
        $($(
            p.insert($key.to_string(), $val.to_string());
        )*)?

        let event = LogEvent::new(
            Some(Participant::new($from_actor, $from_id)),
            Some(Participant::new($to_actor, $to_id)),
            $event_type,
            common_game::logging::Channel::Debug,
            p
        );
        event.emit();
    }};
}

pub const TIMEOUT_DURATION: Duration = Duration::from_millis(2000);

#[cfg(feature = "debug-prints")]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => { println!($($arg)*) };
}

#[cfg(not(feature = "debug-prints"))]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        ()
    };
}
