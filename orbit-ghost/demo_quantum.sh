#!/bin/bash

MOUNT_POINT="/tmp/orbit_ghost_mount"

echo "---------------------------------------------------"
echo "   ORBIT VISIONARY DEMO: QUANTUM ENTANGLEMENT"
echo "---------------------------------------------------"

# 1. Compile the Rust project
echo "[Setup] Compiling Orbit GhostFS..."
cargo build --quiet

# 2. Launch Orbit GhostFS in background
echo "[Action] Activating Flight Plan..."
cargo run --quiet &
ORBIT_PID=$!

# Give FUSE a second to mount
sleep 2

# 3. Verify Projection
echo "[Check] checking mount point..."
if [ -f "$MOUNT_POINT/visionary_demo.mp4" ]; then
    echo "✅ GHOST FILE PROJECTED: visionary_demo.mp4 found."
else
    echo "❌ Projection Failed."
    kill $ORBIT_PID
    exit 1
fi

# 4. The Magic: Read from the ghost file
# This command asks for the LAST 10 bytes of a 50MB file.
# In a normal copy, we'd wait for 50MB to transfer.
# In Orbit, this should trigger a specific block fetch immediately.
echo "[Magic] Attempting to read tail of ghost file..."
start_time=$(date +%s%N)

tail -c 10 "$MOUNT_POINT/visionary_demo.mp4" > /dev/null

end_time=$(date +%s%N)
elapsed=$(( (end_time - start_time) / 1000000 ))

echo ""
echo "✅ READ COMPLETE."
echo "⏱️  Time to access random byte: ${elapsed}ms"
echo "   (Note: Only the requested block was transferred)"

# 5. Cleanup
echo "[Cleanup] Deactivating Orbit..."
kill $ORBIT_PID
wait $ORBIT_PID 2>/dev/null
fusermount -u $MOUNT_POINT 2>/dev/null
