use std::time::Duration;

pub use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use crossbeam_channel::Receiver;
pub use crossbeam_channel::Sender;

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
/// Logs functions internal actions.
///
/// This macro specializes self-directed logging for the Orchestrator (ID 0).
///
/// * `$key => $val` - internal state data to be recorded
/// * `$msg` - shorthand for a simple action description
#[macro_export]
macro_rules! log_internal_op {
    // requires self
    ($self:ident,  $($key:expr => $val:expr),* $(,)? ) => {{
        $crate::log_internal_op!(dir $self.actor_type(), $self.actor_id(), $($key => $val),* )
    }};

    // direct. requires ActorType and ID
    (dir $actor:expr, $id:expr, $($key:expr => $val:expr),* $(,)? ) => {{
        use $crate::{LogEvent, Participant, EventType};

        //selecting actor type
        let event_type=match $actor {
            ActorType::Orchestrator=>{
                EventType::InternalOrchestratorAction
            }
            ActorType::Explorer=>{
                EventType::InternalExplorerAction
            }
            ActorType::Planet=>{
                EventType::InternalPlanetAction
            }
            _=>{
                EventType::InternalOrchestratorAction
                //default case, should not be possible to land here
            }
        };

        LogEvent::self_directed(
            Participant::new($actor, $id),
            event_type,
            $crate::LOG_FN_INT_OPERATIONS,
            $crate::payload!( $($key => $val),* )
        ).emit();
    }};

    // single message (require self)
    ($self:ident, $msg:expr) => {
        $crate::log_internal_op!($self, "action" => $msg );
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
macro_rules! log_fn_call {
    // ----- self: pre-kvs ; result = ... ; post-kvs -----
    ($self:ident, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        $crate::log_fn_call!(
            dir $self.actor_type(),
            $self.actor_id(),
            $fn_name $(, $param)* ;
            $($pre_k => $pre_v),+ ;
            result = $result $(, $($post_k => $post_v),* )?
        )
    }};

    // ----- self: result = ... , post-kvs (no pre) -----
    ($self:ident, $fn_name:expr $(, $param:ident)* ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        $crate::log_fn_call!(
            dir $self.actor_type(),
            $self.actor_id(),
            $fn_name $(, $param)* ;
            result = $result $(, $($post_k => $post_v),* )?
        )
    }};

    // ----- self: only pre-kvs (no result) -----
    ($self:ident, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ $(,)?
    ) => {{
        $crate::log_fn_call!(
            dir $self.actor_type(),
            $self.actor_id(),
            $fn_name $(, $param)* ;
            $($pre_k => $pre_v),+
        )
    }};

    // ----- self: no kvs/result -----
    ($self:ident, $fn_name:expr $(, $param:ident)* $(,)?) => {{
        $crate::log_fn_call!(
            dir $self.actor_type(),
            $self.actor_id(),
            $fn_name $(, $param)*
        )
    }};

    // ----------------- DIR FORMS -----------------

    // dir: pre-kvs ; result = ... ; post-kvs
    (dir $actor:expr, $id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, ActorType, EventType};

        let event_type = match $actor {
            ActorType::Orchestrator => EventType::InternalOrchestratorAction,
            ActorType::Explorer     => EventType::InternalExplorerAction,
            ActorType::Planet       => EventType::InternalPlanetAction,
            _                       => EventType::InternalOrchestratorAction,
        };

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        // pre key-value pairs
        $(
            p.insert($pre_k.to_string(), $pre_v.to_string());
        )+

        // result
        p.insert("Result".to_string(), $result.to_string());

        // post key-value pairs (if any)
        $(
            $(
                p.insert($post_k.to_string(), $post_v.to_string());
            )*
        )?

        LogEvent::self_directed(
            Participant::new($actor, $id),
            event_type,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: result = ... , post-kvs (no pre)
    (dir $actor:expr, $id:expr, $fn_name:expr $(, $param:ident)* ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, ActorType, EventType};

        let event_type = match $actor {
            ActorType::Orchestrator => EventType::InternalOrchestratorAction,
            ActorType::Explorer     => EventType::InternalExplorerAction,
            ActorType::Planet       => EventType::InternalPlanetAction,
            _                       => EventType::InternalOrchestratorAction,
        };

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        p.insert("Result".to_string(), $result.to_string());

        $(
            $(
                p.insert($post_k.to_string(), $post_v.to_string());
            )*
        )?

        LogEvent::self_directed(
            Participant::new($actor, $id),
            event_type,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: only pre-kvs (no result)
    (dir $actor:expr, $id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, ActorType, EventType};

        let event_type = match $actor {
            ActorType::Orchestrator => EventType::InternalOrchestratorAction,
            ActorType::Explorer     => EventType::InternalExplorerAction,
            ActorType::Planet       => EventType::InternalPlanetAction,
            _                       => EventType::InternalOrchestratorAction,
        };

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        $(
            p.insert($pre_k.to_string(), $pre_v.to_string());
        )+

        LogEvent::self_directed(
            Participant::new($actor, $id),
            event_type,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: no kvs/result (original)
    (dir $actor:expr, $id:expr, $fn_name:expr $(, $param:ident)* $(,)?) => {{
        use $crate::{LogEvent, Participant, ActorType, EventType};

        let event_type = match $actor {
            ActorType::Orchestrator => EventType::InternalOrchestratorAction,
            ActorType::Explorer     => EventType::InternalExplorerAction,
            ActorType::Planet       => EventType::InternalPlanetAction,
            _                       => EventType::InternalOrchestratorAction,
        };

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        LogEvent::self_directed(
            Participant::new($actor, $id),
            event_type,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};
}

#[macro_export]
macro_rules! log_orch_to_planet {
    // ===== self FORMS =====

    // self: pre-kvs ; result ; post-kvs
    ($self:ident, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        $crate::log_orch_to_planet!(
            dir $self.actor_id(),
            $fn_name $(, $param)* ;
            $($pre_k => $pre_v),+ ;
            result = $result $(, $($post_k => $post_v),* )?
        )
    }};

    // self: result ; post
    ($self:ident, $fn_name:expr $(, $param:ident)* ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        $crate::log_orch_to_planet!(
            dir $self.actor_id(),
            $fn_name $(, $param)* ;
            result = $result $(, $($post_k => $post_v),* )?
        )
    }};

    // self: only pre
    ($self:ident, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ $(,)?
    ) => {{
        $crate::log_orch_to_planet!(
            dir $self.actor_id(),
            $fn_name $(, $param)* ;
            $($pre_k => $pre_v),+
        )
    }};

    // self: nothing extra
    ($self:ident, $fn_name:expr $(, $param:ident)* $(,)?) => {{
        $crate::log_orch_to_planet!(
            dir $self.actor_id(),
            $fn_name $(, $param)*
        )
    }};

    // ===== DIR FORMS =====

    // dir: pre ; result ; post
    (dir $id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        $(
            p.insert($pre_k.to_string(), $pre_v.to_string());
        )+

        p.insert("Result".to_string(), $result.to_string());

        $(
            $(
                p.insert($post_k.to_string(), $post_v.to_string());
            )*
        )?

        LogEvent::new(
            Some(Participant::new(ActorType::Orchestrator, 0u32)),
            Some(Participant::new(ActorType::Planet, $id)),
            EventType::MessageOrchestratorToPlanet,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: result ; post
    (dir $id:expr, $fn_name:expr $(, $param:ident)* ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        p.insert("Result".to_string(), $result.to_string());

        $(
            $(
                p.insert($post_k.to_string(), $post_v.to_string());
            )*
        )?

        LogEvent::new(
            Some(Participant::new(ActorType::Orchestrator, 0u32)),
            Some(Participant::new(ActorType::Planet, $id)),
            EventType::MessageOrchestratorToPlanet,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: only pre
    (dir $id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        $(
            p.insert($pre_k.to_string(), $pre_v.to_string());
        )+

        LogEvent::new(
            Some(Participant::new(ActorType::Orchestrator, 0u32)),
            Some(Participant::new(ActorType::Planet, $id)),
            EventType::MessageOrchestratorToPlanet,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: nothing extra
    (dir $id:expr, $fn_name:expr $(, $param:ident)* $(,)?) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        LogEvent::new(
            Some(Participant::new(ActorType::Orchestrator, 0u32)),
            Some(Participant::new(ActorType::Planet, $id)),
            EventType::MessageOrchestratorToPlanet,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};
}

#[macro_export]
macro_rules! log_explorer_to_planet {
    // ===== self FORMS =====

    // self: pre-kvs ; result ; post-kvs
    ($self:ident, $explorer_id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        $crate::log_explorer_to_planet!(
            dir $explorer_id,
            $self.actor_id(),
            $fn_name $(, $param)* ;
            $($pre_k => $pre_v),+ ;
            result = $result $(, $($post_k => $post_v),* )?
        )
    }};

    // self: result ; post
    ($self:ident, $explorer_id:expr, $fn_name:expr $(, $param:ident)* ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        $crate::log_explorer_to_planet!(
            dir $explorer_id,
            $self.actor_id(),
            $fn_name $(, $param)* ;
            result = $result $(, $($post_k => $post_v),* )?
        )
    }};

    // self: only pre
    ($self:ident, $explorer_id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ $(,)?
    ) => {{
        $crate::log_explorer_to_planet!(
            dir $explorer_id,
            $self.actor_id(),
            $fn_name $(, $param)* ;
            $($pre_k => $pre_v),+
        )
    }};

    // self: nothing extra
    ($self:ident, $explorer_id:expr, $fn_name:expr $(, $param:ident)* $(,)?) => {{
        $crate::log_explorer_to_planet!(
            dir $explorer_id,
            $self.actor_id(),
            $fn_name $(, $param)*
        )
    }};

    // ===== DIR FORMS =====

    // dir: pre ; result ; post
    (dir $explorer_id:expr, $planet_id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        // params (nome -> Debug)
        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        // pre key-value pairs
        $(
            p.insert($pre_k.to_string(), $pre_v.to_string());
        )+

        // result
        p.insert("Result".to_string(), $result.to_string());

        // post key-value pairs (if any)
        $(
            $(
                p.insert($post_k.to_string(), $post_v.to_string());
            )*
        )?

        LogEvent::new(
            Some(Participant::new(ActorType::Explorer, $explorer_id)),
            Some(Participant::new(ActorType::Planet, $planet_id)),
            EventType::MessageExplorerToPlanet,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: result ; post
    (dir $explorer_id:expr, $planet_id:expr, $fn_name:expr $(, $param:ident)* ;
        result = $result:expr $(, $($post_k:expr => $post_v:expr),* )? $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*

        p.insert("Result".to_string(), $result.to_string());

        $(
            $(
                p.insert($post_k.to_string(), $post_v.to_string());
            )*
        )?

        LogEvent::new(
            Some(Participant::new(ActorType::Explorer, $explorer_id)),
            Some(Participant::new(ActorType::Planet, $planet_id)),
            EventType::MessageExplorerToPlanet,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: only pre
    (dir $explorer_id:expr, $planet_id:expr, $fn_name:expr $(, $param:ident)* ;
        $($pre_k:expr => $pre_v:expr),+ $(,)?
    ) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*


        $(
            p.insert($pre_k.to_string(), $pre_v.to_string());
        )+

        LogEvent::new(
            Some(Participant::new(ActorType::Explorer, $explorer_id)),
            Some(Participant::new(ActorType::Planet, $planet_id)),
            EventType::MessageExplorerToPlanet,
            $crate::LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};

    // dir: nothing extra
    (dir $explorer_id:expr, $planet_id:expr, $fn_name:expr $(, $param:ident)* $(,)?) => {{
        use $crate::{LogEvent, Participant, EventType, ActorType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        $(
            p.insert(stringify!($param).to_string(), format!("{:?}", $param));
        )*


        LogEvent::new(
            Some(Participant::new(ActorType::Explorer, $explorer_id)),
            Some(Participant::new(ActorType::Planet, $planet_id)),
            EventType::MessageExplorerToPlanet,
            $crate::LOG_FN_CALL_CHNL,
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
        use $crate::{LogEvent, Participant};

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


pub trait LoggableActor {
    fn actor_type(&self) -> ActorType;
    fn actor_id(&self) -> u32;
}

pub fn get_sender_id<T>(chan: &Sender<T>) -> usize {
    // getting memory address of the channel
    chan as *const _ as *const () as usize
}
pub fn get_receiver_id<T>(chan: &Receiver<T>) -> usize {
    // getting memory address of the channel
    chan as *const _ as *const () as usize
}