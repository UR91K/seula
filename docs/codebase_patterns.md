# Codebase Patterns

This document outlines common patterns and best practices used throughout the Seula codebase.

## Testing Patterns

### Sequential Test Execution for Shared Resources

**Problem**: Tests that use shared resources (environment variables, files, global state) can interfere with each other when run in parallel, causing flaky test failures.

**Solution**: Use a single `#[test]` function that calls implementation functions in sequence.

**Example**:
```rust
/// Single comprehensive test that runs all scenarios in order
/// This prevents test interference from parallel execution and shared environment variables
#[test]
fn test_all_config_startup_scenarios() {
    println!("=== Running all scenarios in sequence ===");
    
    // Run each test scenario in order
    test_scenario_1_impl();
    test_scenario_2_impl();
    test_scenario_3_impl();
    
    println!("=== All scenarios completed successfully ===");
}

/// Individual test implementations (no #[test] attribute)
fn test_scenario_1_impl() {
    println!("Running: test_scenario_1");
    // Test implementation here...
    
    // Clean up shared resources
    std::env::remove_var("SHARED_ENV_VAR");
}

fn test_scenario_2_impl() {
    println!("Running: test_scenario_2");
    // Test implementation here...
    
    // Clean up shared resources
    std::env::remove_var("SHARED_ENV_VAR");
}
```

**When to Use**:
- Tests that modify environment variables
- Tests that create/modify files in shared locations
- Tests that use global static variables
- Tests that require specific execution order

**Alternative Approaches**:
- `--test-threads=1` flag (slower, affects all tests)
- `Mutex` synchronization (more complex)
- `serial_test` crate (external dependency)

**Used In**: `tests/config_startup_tests.rs` - Config service tests that use `SEULA_CONFIG` environment variable.

---

## Adding an Endpoint

Behaviour lives in a service; the HTTP router is a thin adapter over it (ADR-0018,
ADR-0024). An endpoint is a database method, a service method, a wire type, a handler
and a route, plus tests at the layer where the behaviour is.

### Step-by-Step Process

#### 1. Database method
**File**: `src/database/{domain}.rs`

```rust
impl ProjectDatabase {
    pub fn get_item(&mut self, item_id: &str) -> Result<Option<ItemData>, DatabaseError> {
        // SQL
    }
}
```

#### 2. Service method
**File**: `src/services/{domain}.rs`

The service owns validation and orchestration: checking that what a mutation names
exists, composing several database calls, deciding what an absent row means. Take the
lock once, inside the method.

```rust
pub async fn get_item(&self, item_id: &str) -> Result<Option<ItemRow>, DatabaseError> {
    let mut db = self.db.lock().await;
    db.get_item(item_id)
}
```

Return a `DatabaseError` for failures. `NotFound` becomes a 404 and `InvalidOperation` a
400 at the router; anything else is a 500.

#### 3. Wire types
**File**: `src/http/dto/{domain}.rs`

Hand-written `Serialize` and `Deserialize` structs, not derives on domain types, so the
wire format changes on purpose (ADR-0024). `web/shared/types.ts` mirrors them.

#### 4. Handler
**File**: `src/http/handlers/{domain}.rs`

Parse the request, call one service method, convert the result. No business logic and no
database access. `?` turns a `DatabaseError` into the right status.

```rust
pub async fn get_item(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let item = state
        .services
        .items
        .get_item(&item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Item {} not found", item_id)))?;
    Ok(Json(ItemDto::from(item)))
}
```

#### 5. Route
**File**: `src/http/server.rs`, in `build_router`. A new domain also needs its handler
module declared in `src/http/handlers/mod.rs` and its service added to `Services` in
`src/services/mod.rs`.

#### 6. Tests

- **Behaviour** goes in `tests/services/{domain}.rs`, against the service:
  `let env = test_env();` gives a fresh in-memory library and the services over it.
  Test the happy path, the missing row and the refusal.
- **What only the handler does**, such as status codes, response shape and anything
  built beyond the service's rows, goes in `tests/http/adapters.rs`, through the real
  router: `let (app, env) = app();` then `send(&app, Method::GET, "/api/v1/...")`.

Add a new service test module to `tests/services/mod.rs`.

### Key Patterns & Conventions

- **A service returns rows, not responses.** Counting successes in a batch, naming a scan
  status, embedding a project's tags: the adapter does those.
- **"Not found" is `None` or `NotFound`.** Which one is the service's call; the handler
  maps `None` to a 404 itself.
- **Pagination is the service's.** `limit` and `offset` arrive as `Option<i32>`.
- **Tests that share the database take the lock once.** Hold it across an `.await` on a
  service call and you deadlock.

### Examples in Codebase
- **Tags**: `src/services/tags.rs`, `src/http/handlers/tags.rs`, `src/http/dto/tags.rs`,
  `tests/services/tags.rs`
- **Samples**: `src/services/samples.rs`, `src/http/handlers/samples.rs`
- **Config**: `src/services/config.rs`, `src/http/handlers/config.rs`

---
