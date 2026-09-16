# 0024. An axum HTTP router as a third adapter over the service layer

- **Status:** Accepted — implemented 2026-09-16
- **Recognized:** ADR-0018, which introduced the service layer partly so this surface
  would not become a fourth copy of the business logic
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** ADR-0018 (the service layer and its provision for this router);
  `src/services/mod.rs` (the `Services` aggregator this consumes);
  `src/grpc/handlers/tags.rs` (the adapter shape being copied);
  `src/grpc/error.rs` (the error-conversion precedent); `src/grpc/server.rs`
  (where `SystemService` is constructed outside `Services`); ADR-0023 (which lands first)

## Context

ADR-0018 built `src/services/` because gRPC and the CLI had each implemented the same
operations independently and drifted (ADR-0019 through ADR-0022 are the specific
divergences that audit found). It named an HTTP router as the third caller that made the
duplication worth fixing rather than tolerating.

That router is now being built. The service layer migration it was waiting on is
complete for every domain in the `Services` aggregator.

The consumers are a Tauri application and a local browser. Nothing is planned to cross a
real network. That single fact settles more of the design than anything else here:
it removes authentication, TLS and rate limiting from scope, and it makes CORS a
functional requirement rather than a security one — Tauri's webview and a browser page
are both a different origin from the server, so without a CORS layer the requests simply
do not complete.

## Decision

A new `src/http/` module, structured to mirror `src/grpc/`: a `server.rs` owning the
route table, an `error.rs`, a `state.rs`, a `dto/` directory and a `handlers/` directory
with one file per domain. Handlers are thin in exactly the sense ADR-0018 specified:
parse the request, call one `Services` method, convert the result. No business logic, no
database access.

**State.** An `AppState` bundling `Services` with `SystemService`. The second is needed
because `SystemService` is not a field of the `Services` aggregator — it is constructed
separately in `src/grpc/server.rs` and reached only through `SystemHandler`. `Services`
is already `Clone` over `Arc`-backed fields, so per-request cloning costs what the gRPC
server's own `clone()` per call already costs.

**Wire format.** Hand-written serde structs in `http/dto/`, one module per domain,
independent of the generated proto types.

**Errors.** An `ApiError` with `From<DatabaseError>` and `IntoResponse`, mapping
`NotFound` to 404, validation failures to 400 and everything else to 500. This is the
same move as `impl From<DatabaseError> for tonic::Status` in `src/grpc/error.rs`: it
exists so handlers use `?` and no call site hand-builds an error response.

**Routing.** Resource-oriented under `/api/v1`, with POST sub-resources for the
operations that are not CRUD — batch mutations, rescan, statistics. The gRPC surface is
RPC-shaped throughout; the HTTP surface is not required to mirror its method names.

**Streaming.** The three streaming gRPC endpoints split by kind rather than being
translated uniformly. Scan progress and watcher events become Server-Sent Events, wrapping
the same `ReceiverStream` the gRPC handlers already produce. Media download becomes an
ordinary chunked response with `Content-Type` and `Content-Length`, because it is a file
transfer and not an event stream.

**Transport.** Its own port, from a new `http_port` config field alongside `grpc_port`,
following the existing env-override and validator conventions in `src/config/mod.rs` and
`src/config/validator.rs`. Bound to `127.0.0.1`, like the gRPC server. Spawned as a
second task next to the gRPC server in `src/main.rs`, both over the same `Services` and
`SystemService` instances — one database, one `Arc<Mutex<_>>`, three surfaces.

**CORS.** `tower-http`'s CORS layer, scoped to local origins.

**Logging.** Written against `tracing` from the first line, which is why ADR-0023 lands
first.

Implementation order is incremental for the same reason ADR-0018's was: skeleton and
health check, then tags end-to-end as the pattern proof, then the remaining CRUD domains,
then the streaming endpoints last as the novel and riskiest part.

## Rejected alternatives

- **Derive serde on the generated prost types and reuse them as the HTTP wire format.**
  Less code, and it keeps one definition of each message. Rejected because it makes the
  public JSON shape a function of whatever the `.proto` happens to say, including prost's
  encoding of optionals and enums, which reads badly as hand-written JSON and is
  awkward for a TypeScript consumer. It also couples the two surfaces: a proto change made
  for gRPC's benefit would silently alter the HTTP contract. The DTO duplication is real
  but it is duplication of *shape*, not of logic — the thing ADR-0018 was protecting.
- **OpenAPI-first with `utoipa`, generating a spec and Swagger UI.** Rejected on the
  consumer analysis: the value of a generated spec is mostly in serving clients you do
  not write, and here both clients are first-party and local. The annotation overhead is
  charged on every endpoint forever. Worth revisiting if a third-party consumer ever
  appears — the DTOs are the right place to hang annotations later, so this is not a
  door being closed.
- **Multiplex HTTP and gRPC on one port.** Technically available, since tonic and axum
  both run on hyper. Rejected as complexity bought for nothing: the constraint that
  motivates multiplexing is scarce ports or a single ingress, and neither applies to a
  localhost daemon. Two ports are also independently restartable and trivially
  distinguishable in logs.
- **Translate all three streaming endpoints uniformly, e.g. everything as SSE.**
  Rejected because SSE for a file download is a text-framed wrapper around bytes that
  browsers already know how to stream natively; a plain chunked response is directly
  consumable by an `<audio>` tag or `fetch`, and SSE is not.
- **Defer the streaming endpoints entirely.** Considered and rejected for the plan, but
  only weakly: they are genuinely last in the implementation order, and if a consumer
  need never materializes they can stay unbuilt without affecting anything else here.
- **Add authentication now.** Rejected on the same trust model the gRPC server already
  assumes: loopback-bound, single-user, local machine. Worth revisiting the moment
  anything binds to a non-loopback address, and that is the trigger to watch for — not a
  future decision to add auth, but any change to the bind address.

## Consequences

A third surface exists over the same service layer, which is the outcome ADR-0018 was
designed for — but it also means a service-layer signature change now ripples to three
adapters instead of two. That is the accepted cost of the design, and it is cheaper than
the alternative it replaced, where the logic itself was duplicated.

The DTO layer is real ongoing work: adding a field to a domain means adding it to the
proto, the DTO, and both conversions. This is deliberate, and it is the price of
decoupling the two wire formats.

`AppState` carrying `Services` plus `SystemService` separately makes visible that the
aggregator is incomplete. This ADR does not fix that — folding `SystemService` into
`Services` is a service-layer change, not an HTTP one, and doing it here would mean a
diff that touches gRPC for reasons unrelated to the router.

Two servers now run in the tray process. A failure in one does not stop the other, which
is desirable, but it also means "the daemon is running" becomes an ambiguous statement
and startup logging should say which listeners came up.

## Notes

Planned files are named here without their `src/` prefix — `http/server.rs`,
`http/dto/tags.rs` — deliberately: `tests/docs_tests.rs` checks that every full-form
repo-relative path in `docs/` resolves, and these do not exist yet. They should be
rewritten in full form once they do.

Nothing had been implemented when this was recorded. The decisions above are design
choices made in advance, not reconstructions — if the implementation departs from them,
that departure needs its own ADR rather than a quiet edit to this one.

The cheapest thing to reverse here is the routing convention, which touches only the
route table. The most expensive is the DTO choice, which is load-bearing for every
handler; if it is going to be revisited, revisit it after the tags domain proves the
pattern and before the remaining domains are written.
