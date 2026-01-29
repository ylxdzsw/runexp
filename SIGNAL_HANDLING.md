# Signal Propagation to Spawned Processes

## Overview

This fix ensures that when you press Ctrl-C during concurrent execution, the spawned experiment processes receive the interrupt signal and can handle it appropriately.

## Technical Details

### Unix Systems (Linux, macOS, BSD)

On Unix systems, we use `process_group(0)` to create a new process group for each child process:

```rust
#[cfg(unix)]
{
    child.process_group(0);
}
```

This makes each child process the leader of its own process group. When Ctrl-C is pressed:
- The child process receives SIGINT (signal 2)
- The child can handle the signal (exit, cleanup, or ignore)
- Whether the child exits is up to its own signal handling logic

### Windows Systems

On Windows, we ensure child processes share the parent's console:

```rust
#[cfg(windows)]
{
    child.creation_flags(0);
}
```

This ensures child processes receive CTRL_C_EVENT when Ctrl-C is pressed in the console.

## Testing

### Manual Test

Run the provided test script:

```bash
./test_signal_propagation.sh
```

This will start concurrent experiments that run for 30 seconds. Press Ctrl-C and observe:
- Child processes print "Received signal X in child process!"
- Processes exit with code 130 (standard for SIGINT)

### Expected Behavior

**Before the fix:**
- Press Ctrl-C → runexp exits, but child processes keep running
- Need to manually kill orphaned processes

**After the fix:**
- Press Ctrl-C → runexp exits, child processes receive signal
- Child processes can handle the signal and exit gracefully
- No orphaned processes left running

## Implementation Notes

1. **Zero Dependencies**: This solution uses only Rust standard library features
2. **Cross-Platform**: Works on both Unix and Windows systems
3. **Backward Compatible**: Doesn't change the behavior for normally completing experiments
4. **Process Independence**: Each child can decide how to handle the interrupt signal

## Example Child Process Signal Handling

Python example:
```python
import signal
import sys

def signal_handler(signum, frame):
    print(f"Received signal {signum}, cleaning up...")
    # Perform cleanup
    sys.exit(130)

signal.signal(signal.SIGINT, signal_handler)
```

Bash example:
```bash
#!/bin/bash
trap 'echo "Received SIGINT, cleaning up..."; exit 130' INT

# Your experiment code here
```

The child process can choose to:
- Exit immediately on interrupt
- Perform cleanup before exiting
- Ignore the interrupt and continue (not recommended)
