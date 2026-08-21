# Huddle, audio, TTS and transcription threat model

## Scope and canonical authority

This review covers the boundaries from start/join authorization through native LiveKit media, the Buzz Opus/WebSocket compatibility gateway, local capture/playback, TTS and STT model handling, transcript publication and terminal cleanup. It implements acceptance criteria 14.3, 14.4, 19.1 and 19.2 for CAP-032 under accepted ADR-004.

The target has one huddle model and one native room authority:

- `collaboration_domain::huddle` owns transport-neutral community/channel/huddle-generation identity, lifecycle, participants, roles, moderation, consent state and transcript references. It has no GPUI, LiveKit, device, model or wire dependency.
- `collab` owns trusted tenant, current membership/resource authorization, huddle commands, lifecycle event persistence and transcript projection. Compatibility frames and LiveKit callbacks enter the same reducer.
- `livekit_api`, `livekit_client` and `audio` remain the sole native room/media/device owners. One canonical huddle generation maps to exactly one LiveKit room and scoped token set.
- `collab::huddle::buzz_audio` is a bounded v1/v2 compatibility gateway into that room. Its peer index, version, sequence, jitter and reconnect state are expiring adapter state, never a second roster or durable room.
- `audio::collaboration_tts` owns cancellable local synthesis and verified local model/voice inputs. It may publish audio only as a visibly attributed authorized participant.
- `collab::huddle::transcription` owns consent- and policy-gated bounded partial/final transcript segments, provenance and projection into canonical channel records. Audio frames are not collaboration records.

Recording is not implied by joining, transcription, transport compatibility or model availability. A recording feature would require a separate approved consent, storage and retention design. LiveKit/service/model unavailability is visible and never causes fallback to an independently authoritative Buzz room.

## Source evidence and preserved constraints

- Buzz binds the WebSocket to a host-derived `TenantContext` before upgrade, acquires a global connection permit, caps assembled messages at 8192 bytes, caps Opus frames at 4096 bytes and requires NIP-42 authentication within five seconds.
- Admission enforces relay membership and channel membership, preserves the documented ephemeral-channel auto-add behavior only through an authorized parent, rejects archived channels again after room creation, and defaults absent legacy version to v1 while supporting v1/v2.
- A Buzz room caps 25 peers, allocates one-byte peer indexes, uses eight audio frames (about 160 ms) per peer and a separate 32-entry state-bearing control queue. Audio is deliberately dropped on backpressure; control loss is logged but can desynchronize the legacy peer map.
- v2 frames carry an eight-byte sequence/timestamp/flags/level header. Short frames reject; reserved bits remain opaque; untrusted `level_dbov` is clamped to `-127..=0` and never controls authorization or causes valid audio to drop.
- Buzz heartbeats every 30 seconds and disconnects after three missed pongs. Room end, peer removal and index recycle are serialized; the final peer can win one auto-end transition.
- Buzz cross-pod audio uses Redis-fenced owner generations and reliable control plus lossy datagrams. Stale generations drop. The current media datagram carries a channel/session UUID but not community, so its lookup fails only when same-UUID rooms are simultaneously ambiguous; the target compatibility boundary must use typed community plus generation rather than preserve this omission.
- ADR-004 replaces the Buzz owner/room topology with one LiveKit native room. Legacy v1/v2 connections translate to gateway participants/tracks in the same room; inability to bridge rejects visibly rather than creating a legacy-only room.
- Buzz STT bounds its synchronous audio queue to 50 roughly 100 ms batches and speech to 30 seconds at 16 kHz. It drops overloaded input, validates f32 alignment, uses bounded VAD segments and joins its worker on drop.
- Buzz transcription enable/disable uses a huddle-local generation so in-flight work cannot publish after disable/leave. Missing STT models leave voice working, but worker initialization can currently return a handle that will never emit text; the target must expose this state rather than appear active.
- Buzz TTS pins model revisions/artifact hashes, applies download size limits, rejects archive traversal/symlink/hardlink entries, installs through temp/backup recovery, requires attribution sidecars and enforces a 50-token synthesis chunk.
- Imported reference voices are capped at 25 MiB and 2–30 seconds/8–96 kHz, decoded and canonicalized to content-addressed WAV, written with restricted atomic files, and reverified without following symlinks. Voice audio is biometric/personal content and remains local unless an explicit sharing feature is separately approved.

## Security invariants

1. **HUD-01 — one canonical generation.** Start, join, leave, reconnect, callback, compatibility and end inputs reduce against one typed `(community, channel, huddle generation)` and produce one roster and terminal outcome.
2. **HUD-02 — authorization before room observation.** Trusted tenant, active community, current channel membership, role/resource policy and generation are verified before room/token lookup, join, participant disclosure or media forwarding.
3. **HUD-03 — scoped native tokens.** A LiveKit grant names exactly one room/generation, participant identity, allowed actions and short expiry; it cannot create collaboration membership or be reused across communities/huddles.
4. **HUD-04 — compatibility is not authority.** Buzz v1/v2 peer indexes, pinned versions, Redis ownership and frames are adapter state. Gateway failure closes that path and never starts or preserves an independent room.
5. **HUD-05 — bounded ephemeral media.** Capture, bridge, jitter, encode/decode, fan-out and playback queues have fixed byte/frame/time/peer limits. Overload drops media or disconnects visibly; it never allocates an unbounded backlog.
6. **HUD-06 — untrusted media metadata.** Client sequence, timestamp, flags, level, device, mute and speaking hints cannot authorize membership, moderation, transcript attribution or lifecycle. Canonical identity and server/native track state remain authoritative.
7. **HUD-07 — explicit device permission and control.** Microphone permission, selected input/output, mute/PTT and device loss are visible local states. Denial or loss cannot publish capture frames, silently switch devices or end/fork the huddle.
8. **HUD-08 — audio is not durable by default.** Raw/encoded frames, jitter buffers, levels and device identifiers never enter event storage, transcripts, logs or telemetry. Recording has no implicit path.
9. **HUD-09 — transcription requires current consent and provenance.** Authorized source, visible consent, current generation, participant/time/model provenance, redaction and retention are checked before partial/final publication; cancellation invalidates in-flight output.
10. **HUD-10 — verified local models and voices.** Downloads use pinned source/revision/hash/size and safe atomic install; imported voices are bounded, decoded, canonicalized and path fenced. Failure leaves the last verified source intact.
11. **HUD-11 — cancellable attributed synthesis.** TTS input/output and work are bounded, cancellation silences queued/playing audio, and the published track is attributed to an authorized human/agent without granting that agent wider room authority.
12. **HUD-12 — terminal cleanup and stale fencing.** Leave/end/revocation/cancel closes capture, playback, tracks, subscriptions, bridge state, workers, timers and tokens. Stale callbacks/frames/transcripts cannot reopen or mutate a later generation.

## Threat ledger

| Threat ID | Threat and observable failure | Fail-closed control and canonical owner | Required negative or recovery tests |
| --- | --- | --- | --- |
| T-HUD-001 | Host, channel or legacy frame selects another community | Trusted tenant before upgrade/token; typed community in every room/adapter key | Tasks 13.1, 39.1–39.3, 45.2 |
| T-HUD-002 | Unauthenticated caller probes room/channel existence | Generic pre-upgrade failure and auth before lookup/participant disclosure | Tasks 39.2, 39.3, 45.2 |
| T-HUD-003 | Revoked/nonmember uses an old token or socket | Current membership/generation at token issuance, join and revocation callback | Tasks 39.1–39.3, 39.8 |
| T-HUD-004 | Ephemeral-channel auto-add grants unauthorized parent access | Common policy validates parent membership, relationship and permitted huddle creation | Tasks 13.3, 39.1, 39.3 |
| T-HUD-005 | Archived/ended huddle races join and reopens | Lifecycle generation and terminal state rechecked immediately before native/legacy admission | Tasks 39.1–39.3, 39.8 |
| T-HUD-006 | LiveKit token grants another room, identity or publish capability | Opaque scoped identity, exact room/generation/actions and short expiry | Tasks 39.2, 45.2 |
| T-HUD-007 | Participant changes identity on reconnect/device switch | Canonical principal binding survives reconnect; fresh token cannot retarget identity | Tasks 39.2, 39.4, 39.8 |
| T-HUD-008 | LiveKit webhook/callback for stale room mutates current roster | Verify callback authenticity and map exact room/generation before idempotent reducer | Tasks 39.1, 39.2, 39.8 |
| T-HUD-009 | Legacy version mismatch forks participants into separate room | Gateway returns closed version error; no legacy-only/native-only fallback | Tasks 39.3, 39.8, 43.8 |
| T-HUD-010 | Oversized/malformed text, binary or v2 header exhausts or reaches decoder | WebSocket/frame ceilings and exact v1/v2 parse before bridge allocation | Tasks 39.3, 39.8, 45.4 |
| T-HUD-011 | Client level/timestamp/sequence drives moderation or attribution | Treat as bounded telemetry only; canonical identity/track controls policy | Tasks 39.1, 39.3, 39.8 |
| T-HUD-012 | Peer-index collision attributes audio to wrong speaker | Owner/gateway allocates bounded index per generation and rejects duplicates | Tasks 39.3, 39.8 |
| T-HUD-013 | Audio backlog blocks control/UI or exhausts memory | Independent bounded media/control/jitter queues; media drops; control snapshot repair | Tasks 39.2, 39.3, 39.8, 45.4 |
| T-HUD-014 | Dropped state-bearing legacy control permanently corrupts roster | Revisioned canonical snapshot/delta reconciliation, not queue success, is authority | Tasks 39.1, 39.3, 39.8 |
| T-HUD-015 | N peers create unbounded fan-out/transcode work | Room/bridge/track limits and per-tenant admission/resource budgets | Tasks 4.4, 39.2, 39.3, 45.4 |
| T-HUD-016 | Mesh media lacking community reaches same UUID in another tenant | Target gateway envelope includes typed community/generation and rejects disagreement | Tasks 39.3, 39.8, 45.2 |
| T-HUD-017 | Stale owner/gateway generation continues sending after takeover | Monotonic generation fence and teardown on owner/token/room loss | Tasks 39.2, 39.3, 39.8 |
| T-HUD-018 | Compatibility bridge failure silently isolates legacy clients | Visible join/disconnect/reconnecting failure; never independent Buzz fan-out | Tasks 39.3, 39.7, 39.8 |
| T-HUD-019 | LiveKit outage breaks messaging or falls back to Buzz authority | Huddle admission/media unavailable only; surrounding canonical workspace remains usable | Tasks 39.2, 39.7, 39.8 |
| T-HUD-020 | Microphone permission denial still captures/publishes | Device owner gates capture before track creation; visible denied state | Tasks 39.4, 39.7, 45.2 |
| T-HUD-021 | Device loss/switch silently publishes wrong input or ends room | Stop affected track, retain authorized membership, require explicit/reported recovery | Tasks 39.2, 39.4, 39.7 |
| T-HUD-022 | Mute/PTT race leaks frames after user closes mic | Generation/action fence and capture buffer flush at close; no queued post-mute publish | Tasks 39.4, 39.8 |
| T-HUD-023 | Raw audio, device names or levels leak to event/log/metric storage | Ephemeral typed buffers and static/redacted observability; no frame logging | Tasks 39.2–39.4, 44.5 |
| T-HUD-024 | Recording begins because transcription/agent joins | No recording path; explicit separately approved permission would be mandatory | Tasks 39.1, 39.7, 45.2 |
| T-HUD-025 | Transcript starts without informed current consent | Canonical visible consent command per generation/source before STT | Tasks 39.1, 39.6, 39.7 |
| T-HUD-026 | Disable/leave races in-flight STT and posts afterward | Generation invalidation before worker/task teardown; final publish recheck | Tasks 39.6, 39.8 |
| T-HUD-027 | Transcript text is attributed to wrong participant/agent | Source track-to-principal binding plus segment provenance and authorized projection | Tasks 39.1, 39.6, 39.8 |
| T-HUD-028 | Long/noisy speech, queue pressure or model output exhausts memory | Bounded audio queue, VAD segment, text segment/output and drop/failure state | Tasks 39.5, 39.6, 45.4 |
| T-HUD-029 | Partial transcript duplicates/fabricates a final record | Stable segment identity/version; deterministic replace/final transition and idempotent outbox | Tasks 39.6, 39.8 |
| T-HUD-030 | Transcript bypasses channel visibility, moderation or retention | Canonical authorized message command, provenance, redaction and retention owner | Tasks 19.2, 35.4, 37.2, 39.6 |
| T-HUD-031 | Compromised/truncated model or archive executes/reads arbitrary paths | Pinned hashes/sizes/revisions, safe extraction, no links/traversal and atomic install | Tasks 39.5, 44.5, 45.2 |
| T-HUD-032 | Failed model upgrade deletes last working model/license | Verified temp install, backup rollback, readiness including attribution sidecar | Tasks 39.5, 45.3 |
| T-HUD-033 | Imported voice path traversal/symlink/hash mismatch reads arbitrary file | Content-addressed canonical WAV, restricted root, no symlink and reverify on use | Tasks 39.5, 45.2 |
| T-HUD-034 | TTS text/output or cancellation creates unbounded/leaked playback | Token/output/time/queue bounds and reusable cancellation with drain/silence cleanup | Tasks 39.5, 39.8, 45.4 |
| T-HUD-035 | Agent TTS speaks without identity/permission or gains participant powers | Current participant policy and visible attribution; media publish is narrowly scoped | Tasks 33.3, 39.1, 39.5, 39.7 |
| T-HUD-036 | Leave/end/cancel leaks tracks, threads, subscriptions, bridge peers or credentials | Structured ownership, bounded drain, token revoke/expiry and leak-focused terminal tests | Tasks 39.2–39.8, 45.3 |

## Boundary checklist

### HUD-B01 — lifecycle, policy and generation reducer

- **Owner:** `collaboration_domain::huddle` plus `collab` authorization/command handler.
- **Input:** start/join/leave/end/reaction/moderation/reconnect commands, LiveKit callbacks and compatibility control events.
- **Rule:** derive trusted tenant/principal, load current membership/role/resource, bind one generation and apply idempotently. Authorization precedes room existence and participant observation.
- **Tests:** Tasks 39.1, 39.8 and 45.2.

### HUD-B02 — native LiveKit token and room

- **Owner:** `audio::collaboration_huddle`, `livekit_api` and `livekit_client`.
- **Rule:** exact one-room-per-generation mapping; short-lived participant/action token; authenticated callbacks; current lifecycle reconciliation on connect/reconnect/end.
- **Failure:** token/room/service failure is scoped and visible. No Buzz-room fallback and no durable state in LiveKit callbacks.
- **Tests:** Tasks 39.2, 39.8 and 45.3.

### HUD-B03 — Buzz v1/v2 WebSocket gateway

- **Owner:** `collab::huddle::buzz_audio` as a compatibility adapter.
- **Order:** host tenant → global capacity → bounded upgrade → five-second NIP-42 → relay/channel policy → active generation/version → LiveKit gateway participant → bounded frame bridge.
- **State:** peer index, version, heartbeat, jitter and sequence expire with connection/generation; canonical snapshot repairs lost control deltas.
- **Tests:** Tasks 39.3, 39.8 and 43.8.

### HUD-B04 — media codec, bridge and backpressure

- **Owner:** native audio/LiveKit adapter and the bounded compatibility codec bridge.
- **Rule:** exact supported Opus parameters and v1/v2 headers; bounded decode/transcode/fan-out/jitter queues; malformed input rejects before codec; media pressure drops frames rather than blocking state/control.
- **Observability:** only coarse static reason/quality/resource classes; no frames, participant IDs or device labels in metric dimensions.
- **Tests:** Tasks 39.2, 39.3, 39.8 and 45.4.

### HUD-B05 — capture, playback and device control

- **Owner:** existing Zed `audio` device abstractions and native huddle controls.
- **Rule:** permission before capture, explicit selected devices, mute/PTT/action generation at buffer/track boundary, and visible device loss/switch/retry.
- **Failure:** stop the affected track and clear queued capture/playback without changing membership unless lifecycle policy says so.
- **Tests:** Tasks 39.4, 39.7 and 39.8.

### HUD-B06 — TTS model acquisition and readiness

- **Owner:** Zed model/cache conventions consumed by `audio::collaboration_tts`.
- **Rule:** allowlisted HTTPS source and pinned revision/hash/size; streamed byte ceiling; traversal/link-safe extraction; verified temp install; attribution and manifest are readiness requirements.
- **Failure:** keep/restore prior verified model; expose unavailable/error; never execute a partial directory.
- **Tests:** Tasks 39.5, 44.5 and 45.3.

### HUD-B07 — imported voice custody and synthesis

- **Owner:** local audio/TTS service.
- **Input:** user-selected bounded regular audio file and bounded text/voice choice.
- **Rule:** decode/canonicalize, validate duration/rate/finiteness, content-address, atomically restrict and reverify without symlinks; synthesize through bounded chunks/output and cancellation.
- **Privacy:** imported voice stays device-local. Names/paths/samples are excluded from collaboration logs/telemetry/events.
- **Tests:** Tasks 39.5, 39.8 and 45.2.

### HUD-B08 — STT capture and consent

- **Owner:** local STT/audio source plus canonical huddle consent state.
- **Rule:** show consent/transcription state, accept audio only from authorized current generation/source, bound queues/VAD segments/model work and stop immediately on disable/leave/revocation.
- **Failure:** missing/dead model is visibly unavailable and retryable; voice media continues without pretending transcription is active.
- **Tests:** Tasks 39.6, 39.7 and 39.8.

### HUD-B09 — transcript segment and channel projection

- **Owner:** `collab::huddle::transcription` and canonical message/retention owners.
- **Rule:** stable segment/source/generation identity, partial/final version, participant/time/model/provider/consent provenance, bounded text, policy/redaction and one message/outbox transaction.
- **Failure:** stale/duplicate/late output drops or reconciles; projection failure retries idempotently and cannot fabricate final text.
- **Tests:** Tasks 19.2, 35.4, 37.2, 39.6 and 39.8.

### HUD-B10 — terminal cleanup, deployment and compatibility retirement

- **Owner:** structured huddle session owner, deployment/runtime and Phase 8 compatibility gate.
- **Rule:** cancel and join/close capture, playback, tracks, subscriptions, adapters, worker threads, timers and tokens; reconcile orphan participants after restart; validate secrets/routes/limits/readiness.
- **Compatibility:** adapter supports v1/v2 until ADR-004 retirement criteria and release approval are met. Removal drains new legacy sessions before routing changes and retains rollback to the gateway against canonical rooms only.
- **Tests:** Tasks 39.8, 44.3–44.5, 45.3 and 48.2.

## Known gaps and strengthening obligations

1. Buzz's legacy room and Redis owner are complete media authorities. ADR-004 deliberately prevents porting them as peers: Task 39.3 must preserve their observable safety/wire behavior while translating into LiveKit and deleting any path that can sustain a legacy-only room.
2. The Buzz media datagram lacks community identity and relies on unambiguous channel UUID lookup. Ambiguity drops safely, but absence of a collision is not tenant authentication. The target adapter envelope must carry typed community plus generation and validate both at every hop.
3. Buzz can drop a full state-bearing legacy control queue and only warn that the peer map may desynchronize. The canonical reducer/snapshot is the repair authority; Task 39.3 must not treat delivery of every delta as guaranteed.
4. Buzz's v1/v2 room pins one version for the generation. The compatibility adapter must retain documented version errors without making protocol version the canonical room identity; mixed native/legacy evidence in Task 39.8 is required.
5. Buzz STT can return a pipeline handle even when its worker exits because model initialization failed, and some queue/thread outcomes are intentionally dropped. The target must expose dead/unavailable state, clear it for retry and prove teardown instead of silently showing transcription active.
6. Audio/TTS/STT and model code uses several best-effort drops and cleanup paths appropriate to ephemeral media. Canonical lifecycle/transcript errors may not be discarded: state-changing failures must be surfaced, retried idempotently or recorded for recovery.
7. Model hashes validate known bytes but do not make upstream model behavior trusted. Inputs/outputs/resources remain bounded and models never receive credentials, unrestricted paths, network access or authorization authority.
8. Imported reference voices may be biometric/personal data. Current Buzz storage is device-local; sharing, server backup, cross-device sync or training use is not approved by this spec and requires an explicit product/privacy decision.
9. Numeric compatibility values in this document are evidence, not automatically target budgets. Task 4.4 assigns canonical owners, metrics, alerts and verification before readiness.

## Cross-cutting verification checklist

- **Authorization/isolation:** Tasks 13.3, 39.1–39.3, 39.8 and 45.2 cover unknown hosts, wrong community/channel/generation, nonmember/revoked principals, stale tokens/callbacks and same-UUID tenant probes.
- **Native/legacy equivalence:** Tasks 39.1–39.3 and 39.8 compare start/join/leave/end, roster, reconnect, version, malformed frame, bridge outage and one terminal outcome.
- **Media/resource safety:** Tasks 39.2–39.4, 39.8 and 45.4 cover peer/frame/queue/jitter/transcode limits, backpressure, device permission/loss and cancellation leaks.
- **Privacy/observability:** Tasks 39.2–39.7, 44.5 and 45.2 prove no raw audio/device/model/voice/transcript content in unauthorized storage, logs, metrics or errors.
- **Models/TTS:** Tasks 39.5, 39.8 and 45.3 cover missing/corrupt/oversized models, safe archive paths, hash/license/readiness, interrupted upgrade, imported voice fencing, output limits and cancellation.
- **Transcription:** Tasks 39.1, 39.6–39.8 and 37.2 cover consent, disable/leave races, source attribution, partial/final idempotency, retry, redaction and retention expiry.
- **Cleanup/compatibility:** Tasks 39.2–39.8, 44.3–44.5, 45.3 and 48.2 cover service/adapter restart, orphan roster repair, room/token/device/model failure, resource release and retirement gates.

Task 4.4 must consume every numeric bound named here. The final security, recovery and scale gates supplement rather than replace each focused negative test above.
