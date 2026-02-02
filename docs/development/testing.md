# Testing

Anvil has both frontend and backend test suites. Frontend tests use Vitest with jsdom; backend tests use Rust's built-in test framework with `cargo test`.

## Frontend Tests

### Running

```bash
npm run test          # Run all frontend tests once
npm run test:watch    # Run in watch mode (re-runs on file change)
```

### Configuration

Frontend tests are configured in `vitest.config.ts`:

- **Environment**: jsdom (simulates a browser DOM).
- **Setup file**: `src/__tests__/setup.ts` (runs before each test suite).
- **Test location**: `src/__tests__/` directory, organized by domain.

### Tauri API mocks

Since tests run outside the Tauri runtime, all Tauri APIs are mocked in `src/__mocks__/`:

| Mock | Behavior |
|------|----------|
| `invoke` | Returns empty arrays or default values for all commands. |
| Event listeners (`listen`, `once`) | No-ops that return cleanup functions. |
| `window.matchMedia` | Mocked to support theme detection tests. |

These mocks allow store and component tests to run without a Tauri backend.

### Test organization

Tests are stored in `src/__tests__/stores/`:

| Test file | Store tested | What it covers |
|-----------|-------------|----------------|
| `moduleStore.test.ts` | taskStore | Task listing, state detection, execution flow |
| `themeStore.test.ts` | themeStore | Theme toggling (light/dark/system), resolved theme |
| `toastStore.test.ts` | toastStore | Adding/removing toasts, auto-dismiss behavior |

### Current coverage

The frontend test suite contains 16 tests across 3 store test files.

### Writing frontend tests

Store tests interact directly with the Zustand store via `getState()` and action methods:

```typescript
import { useTaskStore } from '@/stores/taskStore';

describe('taskStore', () => {
  beforeEach(() => {
    useTaskStore.getState().reset();
  });

  it('should start with empty tasks', () => {
    const state = useTaskStore.getState();
    expect(state.tasks).toEqual([]);
  });
});
```

## Backend Tests

### Running

```bash
cd src-tauri && cargo test
```

### Current coverage

The backend test suite contains 59 tests.

### Test organization

Backend tests use inline `#[test]` and `#[tokio::test]` functions co-located with the source code. Each module file contains its own tests in a `#[cfg(test)] mod tests` block.

| Module | Test file | What it covers |
|--------|-----------|----------------|
| AppArmor log parser | `apparmor/log_parser.rs` | Standard denial lines, missing fields, non-denial lines, edge cases |
| AppArmor rule generator | `apparmor/rule_generator.rs` | Rule generation from denial patterns, risk level assignment |
| AppArmor rule consolidator | `apparmor/rule_consolidator.rs` | Rule merging, glob patterns, deduplication |
| Blueprint defaults | `blueprints/defaults.rs` | Unique IDs, is_builtin flag, expected task counts, order values |
| Blueprint manager | `blueprints/manager.rs` | Add, update, remove, create, save/reload, builtin protection, error cases |
| Config manager | `config/manager.rs` | Default values, serialization roundtrip, partial deserialization, save/load |
| Local executor | `executor/local.rs` | Command execution, file existence check, is_local, file read/write |
| Fleet manager | `fleet/manager.rs` | Add, update, remove, save/reload, local node detection, error cases |

### Filesystem isolation

Backend tests that involve file I/O use the `tempfile` crate to create temporary directories:

```rust
#[test]
fn save_and_reload_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let mut mgr = test_manager(dir.path());

    let bp = make_blueprint("persistent", false);
    let id = bp.id.clone();
    mgr.add_blueprint(bp).unwrap();

    // Create a new manager pointing at the same temp dir
    let mut mgr2 = test_manager(dir.path());
    mgr2.load_blueprints_from_disk().unwrap();

    assert_eq!(mgr2.get_blueprint(&id).unwrap().name, "persistent");
}
```

This approach ensures tests never read from or write to real config directories (`~/.config/anvil/`).

### Writing backend tests

Place tests in a `#[cfg(test)]` module at the bottom of the source file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn your_test_name() {
        // Arrange
        let dir = tempdir().unwrap();

        // Act
        // ... call your functions ...

        // Assert
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn your_async_test() {
        // For async functions
        let result = some_async_function().await;
        assert!(result.is_ok());
    }
}
```

Use `tempfile::tempdir()` whenever you need a temporary directory. The directory and its contents are automatically deleted when the `TempDir` value is dropped.
