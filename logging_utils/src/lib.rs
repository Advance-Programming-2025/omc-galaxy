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
/// To be used only when the orchestrator receives the ack

// ---------------------------------------------------------------------------------------
// LOG Macros
// ---------------------------------------------------------------------------------------


/// Creates a BTreeMap payload from key-value pairs for use in log events.
///
/// This macro simplifies the creation of structured metadata for logging by automatically
/// converting keys and values to strings and inserting them into a BTreeMap.
///
/// # Example usage
/// ```
/// let data = payload!(
///     "planet_status" => "active",
///     "energy_cells" => 5,
///     "cell_index" => current_idx,
/// );
/// ```
///
/// # Arguments
/// * `$key => $val` - Any number of key-value pairs where both key and val will be converted to String

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
/// This macro captures comprehensive error context including the warning category,
/// specific error details, function name, local variables, and optional custom metadata.
///
/// # Usage
/// ```
/// // Basic usage for a failed resource combination
/// warning_payload!("Resource combination failed", err, "make_water", hydrogen, oxygen)
///
/// // With additional context metadata
/// warning_payload!(
///     "Rocket construction failed",
///     build_err,
///     "build_rocket",
///     cell_idx;
///     "..." => "...",
///     "available_energy" => energy
/// )
/// ```
///
/// # Arguments
/// * `$warn` - High-level warning category or message
/// * `$err` - The specific error value or exception
/// * `$func` - Name of the function where the error occurred
/// * `$param` - Zero or more variable identifiers to capture (will be Debug-formatted)
/// * `$key => $val` - Optional additional key-value pairs (after semicolon)
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
/// Logs internal actor actions and state changes.
///
/// This macro specializes in self-directed logging for any actor.
/// It records internal operations that do not involve external communication.
/// It supports two modes: one using `self` to automatically extract actor info, and a
/// direct mode where actor type and ID are explicitly provided.
///
/// # Usage
/// ```
/// // Using self (e.g., inside Planet)
/// log_internal_op!(self, "cell_recharged" => "success", "new_charge" => 1);
/// log_internal_op!(self, "Sunray processed");
///
/// // Direct mode (e.g., logging for a specific explorer from the orchestrator)
/// log_internal_op!(dir ActorType::Explorer, exp_id, "action" => "handle_asteroid", "available_energy_cells" =>eergy_cells_idx );
/// ```
///
/// # Arguments
/// ## Self mode:
/// * `$self` - The actor instance (must implement LoggableActor)
/// * `$key => $val` - Key-value pairs describing the internal state/operation
/// * `$msg` - Single string message (shorthand for "action" => message)
///
/// ## Direct mode (prefix with `dir`):
/// * `$actor` - ActorType enum value
/// * `$id` - Numeric ID of the actor
/// * `$key => $val` - Key-value pairs describing the internal state/operation
///
/// # Channel
/// Logs to LOG_FN_INT_OPERATIONS (Trace level)
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
/// Records function execution, input arguments, and execution results.
///
/// This macro supports advanced tracing by allowing metadata to be captured
/// before the result and additional context after the result is determined.
/// It supports both self-based invocation (using LoggableActor trait) and
/// direct invocation with explicit actor information.
///
/// # Usage Patterns
/// ```
/// // Simple: just function name and parameters
/// log_fn_call!(self, "handle_sunray", sunray,);
///
/// // With pre-execution metadata only
/// log_fn_call!(self, "on_explorer_arrival", explorer_id; "protocol" => "TravelToPlanetRequest", "status" => "Incoming");
///
/// // With result only
/// log_fn_call!(self, "handle_asteroid", asteroid; result = rocket_deflection);
///
/// // With result and post-execution metadata
/// log_fn_call!(self, "make_basic_resource", resource_type; result = resource_out, "energy_cells_discharged" => 1, "remaining_energy" => current_charge);
///
/// // Full form: pre-metadata, result, and post-metadata
/// log_fn_call!(
///     self, "handle_combination", complex_req;
///     "ingredients_checked" => true;
///     result = combination_result,
///     "energy_consumed" => 1,
///     "bag_updated" => is_success
/// );
///
/// // Direct mode (without self)
/// log_fn_call!(dir ActorType::Planet, planet_id, "handle_internal_state_req"; result = dummy_planet_state);
/// ```
///
/// # Arguments
/// ## Self mode:
/// * `$self` - The actor instance (must implement LoggableActor)
/// * `$fn_name` - Name of the function being logged
/// * `$param` - Zero or more parameter identifiers to capture (will be Debug-formatted)
/// * `$pre_k => $pre_v` - Optional pre-execution key-value pairs (before result)
/// * `result = $result` - Optional result value to log
/// * `$post_k => $post_v` - Optional post-execution key-value pairs (after result)
///
/// ## Direct mode (prefix with `dir`):
/// * `$actor` - ActorType enum value
/// * `$id` - Numeric ID of the actor
/// * (remaining arguments same as self mode)
///
/// # Output Structure
/// Creates a log event with payload containing:
/// - "fn": function name
/// - parameter names: Debug-formatted parameter values
/// - pre-execution key-value pairs (if provided)
/// - "Result": result value (if provided)
/// - post-execution key-value pairs (if provided)
///
/// # Channel
/// Logs to LOG_FN_CALL_CHNL (Debug level)
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
/// Logs messages sent from the Orchestrator to a Planet.
///
/// This macro creates log events for Orchestrator-to-Planet communication, capturing
/// function calls and message passing between these actors. Supports multiple formats
/// for different levels of detail.
///
/// # Usage Patterns
/// ```
/// // Simple: just function name and parameters
/// log_orch_to_planet!(self, "send_sunray", planet_id, sunray);
///
/// // With pre-execution metadata
/// log_orch_to_planet!(self, "incoming_explorer_request", explorer_id; "assigned_sender" => "active");
///
/// // With result
/// log_orch_to_planet!(self, "internal_state_request"; result = dummy_planet_state);
///
/// // Full form: pre-metadata, result, and post-metadata
/// log_orch_to_planet!(
///     self, "send_asteroid", planet_id, asteroid;
///     "event_type" => "MessageOrchestratorToPlanet";
///     result = asteroid_ack,
///     "rocket_deflected" => has_rocket
/// );
///
/// // Direct mode (specify planet ID explicitly)
/// log_orch_to_planet!(dir planet_id, "start_planet_ai"; result = "AI_Running");
/// ```
///
/// # Arguments
/// ## Self mode:
/// * `$self` - The orchestrator instance (must implement LoggableActor with actor_id returning planet ID)
/// * `$fn_name` - Name of the function/message being logged
/// * `$param` - Zero or more parameter identifiers to capture
/// * `$pre_k => $pre_v` - Optional pre-execution key-value pairs
/// * `result = $result` - Optional result value
/// * `$post_k => $post_v` - Optional post-execution key-value pairs
///
/// ## Direct mode (prefix with `dir`):
/// * `$id` - Planet ID (recipient)
/// * (remaining arguments same as self mode)
///
/// # Event Details
/// - From: Orchestrator (ID: 0)
/// - To: Planet (ID: specified or from self.actor_id())
/// - EventType: MessageOrchestratorToPlanet
/// - Channel: LOG_FN_CALL_CHNL (Debug level)
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
/// Logs messages sent from an Explorer to a Planet.
///
/// This macro creates log events for Explorer-to-Planet communication. Supports multiple formats
/// for varying levels of detail.
///
/// # Usage Patterns
/// ```
/// // Simple: just function name and parameters
/// log_explorer_to_planet!(self, explorer_id, "available_energy_cell_request");
///
/// // With pre-execution metadata
/// log_explorer_to_planet!(self, explorer_id, "supported_resource_request"; "cache_available" => false);
///
/// // With result
/// log_explorer_to_planet!(self, explorer_id, "generate_resource_request", resource_type; result = generate_response);
///
/// // Full form: pre-metadata, result, and post-metadata
/// log_explorer_to_planet!(
///     self, explorer_id, "combine_resource_request", complex_resource_request;
///     "ingredients_present" => true;
///     result = combination_outcome,
///     "energy_discharged" => true,
///     "resource_type" => "Complex"
/// );
///
/// // Direct mode (specify both IDs explicitly)
/// log_explorer_to_planet!(dir explorer_id, planet_id, "on_explorer_departure"; result = "success");
/// ```
///
/// # Arguments
/// ## Self mode:
/// * `$self` - The planet instance (must implement LoggableActor with actor_id returning planet ID)
/// * `$explorer_id` - ID of the explorer sending the message
/// * `$fn_name` - Name of the function/message being logged
/// * `$param` - Zero or more parameter identifiers to capture
/// * `$pre_k => $pre_v` - Optional pre-execution key-value pairs
/// * `result = $result` - Optional result value
/// * `$post_k => $post_v` - Optional post-execution key-value pairs
///
/// ## Direct mode (prefix with `dir`):
/// * `$explorer_id` - Explorer ID (sender)
/// * `$planet_id` - Planet ID (recipient)
/// * (remaining arguments same as self mode)
///
/// # Event Details
/// - From: Explorer (ID: specified)
/// - To: Planet (ID: specified or from self.actor_id())
/// - EventType: MessageExplorerToPlanet
/// - Channel: LOG_FN_CALL_CHNL (Debug level)
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