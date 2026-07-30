# Adapter Protocol 3.0.0 Conformance Matrix

Authority: `/Users/daniel/workspace/learn/tui-tetris/protocol/adapter`.

Status values describe the implementation after the v3 migration. Test names
refer to `tests/adapter_tcp.rs` unless another file is named.

| Protocol requirement | Current implementation | Difference | Test evidence |
| --- | --- | --- | --- |
| Report 3.0.0 and reject incompatible majors | Strict `semver::Version` parsing; server major must match | Aligned | `hello_receives_welcome`, `hello_with_mismatched_major_receives_protocol_mismatch`, `malformed_semver_receives_protocol_mismatch` |
| First valid message is hello; `hello.seq=1` | Pre-handshake dispatch accepts only hello and validates sequence | Aligned | `command_before_hello_receives_handshake_required`, `control_before_hello_receives_handshake_required`, `hello_requires_sequence_one_and_json_format` |
| Hello formats contain JSON | Explicit membership check | Aligned | `hello_requires_sequence_one_and_json_format` |
| Hello includes required client identity | Typed hello parsing requires `client.name` and `client.version` | Aligned | `hello_requires_client_identity` |
| Welcome identity and role fields | Emits stable connection `client_id`, assigned `role`, and current `controller_id` | Aligned | `hello_receives_welcome` |
| Capability partitions and controller policy | Emits union, always/optional partitions, and lowest-client-id promotion policy | Aligned | `welcome_includes_required_capabilities` |
| Explicit observer is not auto-assigned or promoted | Requested role retained per connection and excluded from automatic promotion | Aligned | `observer_request_is_never_auto_promoted` |
| One controller; claim/release and disconnect cleanup | Release clears ownership; explicit claim reassigns; eligible observer promoted on disconnect | Aligned | `release_clears_controller_until_observer_claims`, `eligible_observer_is_promoted_after_controller_disconnect`, `observer_command_receives_not_controller` |
| Strictly increasing post-welcome client sequence | Per-connection last accepted sequence is enforced | Aligned | `duplicate_or_decreasing_sequence_is_rejected` |
| Action list has at most 32 entries | Parse validation rejects larger lists | Aligned | `action_count_and_restart_payload_are_validated` |
| Restart parameters require restart action | Parse validation enforces coupling | Aligned | `action_count_and_restart_payload_are_validated` |
| Seeded restart is deterministic | Core reset accepts a u32 seed and rebuilds the complete RNG stream | Aligned | `seeded_restart_is_deterministic_and_starts_new_episode` |
| Place failures are atomic | Planning and application happen on a cloned state; commit occurs only after success | Aligned | `invalid_place_is_atomic` |
| Place origin stays within schema range | Parsing rejects `place.x` outside -128 through 127 before planning | Aligned | `place_origin_outside_schema_range_is_invalid_command` |
| Place rotation uses the schema enum exactly | Rotation parsing accepts only lowercase `north/east/south/west` | Aligned | Adapter unit test `decode_rotation_requires_schema_case` |
| Game-command ack contains correlation and authoritative result | Ack includes `correlation_seq`, `applied_step`, and post-application `state_hash` | Aligned | `controller_command_receives_ack_and_applies_action` |
| Control ack omits game-state result | Control ack contains correlation only | Aligned | `release_clears_controller_until_observer_claims` |
| Full observation with authoritative logical step | Core owns and advances `logical_step` for accepted actions and active fixed ticks | Aligned | `first_observation_is_full_snapshot`, `controller_command_receives_ack_and_applies_action` |
| Stable episode/piece/step identities | Core increments episode only on restart, piece only on active-piece replacement, and step within the current piece | Aligned | `seeded_restart_is_deterministic_and_starts_new_episode`, `piece_id_changes_only_when_active_piece_changes` |
| Board cell values are 0..7 | Empty is 0; I/O/T/S/Z/J/L map to 1..7 | Aligned | `first_observation_is_full_snapshot` |
| `board_id` changes only with locked-board changes | Uses the monotonic board revision, incremented on lock/clear and restart | Aligned | `board_id_changes_only_when_locked_board_changes` |
| Exactly five next pieces and `next=next_queue[0]` | Adapter publishes the first five from a core queue maintained at five or more | Aligned | `first_observation_is_full_snapshot`, `tests/queue_size.rs` |
| Ordered bounded events array, never null | Core records real lock results; adapter drains at most four in causal order | Aligned | `lock_event_reports_actual_clear_and_score`, `events_are_ordered_bounded_and_never_null`, `first_observation_is_full_snapshot` |
| State hash is a 16-character lowercase digest | FNV-1a-style implementation-local digest covers gameplay state, lifecycle flags, timers, queue, board, and seed | Aligned | `first_observation_is_full_snapshot`, `controller_command_receives_ack_and_applies_action`, adapter unit hash test |
| Full snapshot immediately after streaming hello | Handshake marks the client snapshot-due and bypasses observation throttling | Aligned | `first_observation_is_full_snapshot` |
| Frames up to 65,536 bytes; bounded incomplete frames | Read buffer is capped, accepts the exact limit, and disconnects an oversized unterminated client | Aligned | `frame_at_exact_payload_limit_is_accepted`, `unterminated_oversized_frame_closes_only_that_client` |
| Bounded outbound buffering and slow-client isolation | Responses have a hard byte cap; observations use current-plus-latest coalescing slots | Aligned | Structural implementation review; closed-loop and oversized-client isolation tests |
| Passive observers remain stream-capable | Inbound idle timeout applies only to incomplete handshakes and controllers | Aligned | `observer_is_not_disconnected_by_inbound_idle_timeout` |
| Reconnect and closed-loop stability | Listener remains live across controller reconnects | Aligned | `closed_loop_stability_reconnect_smoke` |
| Portable black-box conformance | Release bundle accepts a real authority client outside the test harness | Aligned | Authority `adapter_verify.py all`: ready, claim, restart, determinism |

Run the repository gates in `AGENTS.md`; the TCP suite may require loopback
socket permission.
