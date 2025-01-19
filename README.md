# DTChat - SharedPeer Refactor Branch

## Overview

I introduced a critical refactoring to the **DTChat** application, enhancing the management of `Peer` instances through shared ownership and mutable access. Leveraging Rust's `Rc<RefCell<Peer>>` (referred to as `SharedPeer`), the application achieves more efficient state management, reduces data duplication(fosters consistent data synchronization) across multiple UI components.

## Modifications

### 1. Declared `SharedPeer` in `peer_config.rs`

**File:** `src/peer_config.rs`

**Changes:**

- Introduced a type alias `SharedPeer` defined as `Rc<RefCell<Peer>>`.
- Facilitated shared ownership and mutable access to `Peer` instances across various modules.

**Import:**
```rust
use std::rc::Rc;
use std::cell::RefCell;
```

### 2. Changed the remaining files based on the `SharedPeer` logic

**Changes:**

- Made modifications based on the new RC logic