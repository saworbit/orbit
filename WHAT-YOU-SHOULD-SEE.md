# Visual Verification Guide

## 🎯 What You SHOULD See (New Chunk Map)

When you click a job, you should see:

### Header Section
```
← Back    |    Job #123    [RUNNING]
```

### Left Panel: Configuration
```
┌─────────────────────────────┐
│ Configuration               │
├─────────────────────────────┤
│ 📁 Source                   │
│ C:\Windows\System32         │
│                             │
│ 📁 Destination              │
│ C:\Temp\test                │
│                             │
│ ┌──────┬──────┬──────┐      │
│ │Comp. │Verif.│Worker│      │
│ │ ✓    │ ✓    │  4   │      │
│ └──────┴──────┴──────┘      │
└─────────────────────────────┘
```

### Right Panel: THE CHUNK MAP (This is the new part!)
```
┌─────────────────────────────────────────────────────┐
│ Chunk Allocation Map              4660/10240 Chunks │
├─────────────────────────────────────────────────────┤
│                                                     │
│  🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜            │
│  🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜            │
│  🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜            │
│  🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜            │
│  🟢🟢🟢🟢🟢🟢🟢🟢🟢🟢 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜            │
│                                                     │
│  🟢 Synced    🔴 Failed                             │
└─────────────────────────────────────────────────────┘
```

**Key visual indicators:**
- ✅ Green blocks with GLOW effect (shadow around them)
- ✅ 20 columns × 5 rows = 100 blocks total
- ✅ Smooth animations when blocks change color
- ✅ Progress bar at top showing percentage

---

## ❌ What You're Probably Seeing (Old Version)

If you see **text-only job details** like this:

```
Job #123
Status: running
Progress: 45%

[Some basic text info, NO visual grid]
```

**Then you're viewing CACHED old code!**

---

## 🔍 Quick Visual Check

**Look for these exact things:**

1. **Grid layout** - Should see a grid of tiny squares (20 across)
2. **Green glowing blocks** - NOT just solid color, they should have a subtle glow
3. **"Chunk Allocation Map"** heading
4. **Legend at bottom** showing "🟢 Synced  🔴 Failed"

If you see ALL of these → ✅ You have the new version!
If you see NONE of these → ❌ You have cached old code

---

## 🚀 How to Fix

### Option 1: Dev Mode (Recommended)
```batch
.\fix-ui-cache.bat
```

Then open **INCOGNITO window** to http://localhost:5173

### Option 2: Production Mode
```batch
cd dashboard
npm run build
cd ..
cargo run --release -p orbit-server --features ui
```

Then go to http://localhost:8080

### Option 3: Nuclear Option
```batch
# Close ALL Orbit windows
# Close browser completely
# Delete magnetar.db (to start fresh)
del magnetar.db
del orbit-server-users.db

# Restart everything
.\launch-orbit.bat
```

Open browser INCOGNITO window → http://localhost:5173

---

## 📸 Screenshot Comparison

### OLD (Text-based)
```
╔══════════════════════════════╗
║ Job Details                  ║
╠══════════════════════════════╣
║ ID: 123                      ║
║ Status: Running              ║
║ Progress: 45%                ║
║ Source: C:\test              ║
║ Dest: C:\output              ║
╚══════════════════════════════╝
```

### NEW (Visual Chunk Map)
```
╔═══════════════════════════════════════════════════╗
║  ← Back  |  Job #123  [RUNNING]                   ║
╠═══════════════════════════════════════════════════╣
║                                                   ║
║  ┌─────────────┐  ┌──────────────────────────┐   ║
║  │ Config      │  │ 45.5%  | 4660 | 12       │   ║
║  │ Source: C:\ │  │ Total  |Synced|Failed    │   ║
║  │ Dest: C:\   │  └──────────────────────────┘   ║
║  │             │                                  ║
║  │ Compress: ✓ │  Chunk Allocation Map            ║
║  │ Verify: ✓   │  ┌─────────────────────────┐    ║
║  │ Workers: 4  │  │ 🟢🟢🟢🟢🟢🟢⬜⬜⬜⬜⬜⬜⬜⬜⬜│    ║
║  └─────────────┘  │ 🟢🟢🟢🟢🟢🟢⬜⬜⬜⬜⬜⬜⬜⬜⬜│    ║
║                   │ (10 rows of 20 blocks)   │    ║
║                   └─────────────────────────┘    ║
╚═══════════════════════════════════════════════════╝
```

**If you see the NEW layout → SUCCESS! 🎉**
**If you see the OLD layout → Run fix-ui-cache.bat**

---

## 🆘 Still Not Working?

1. Check you're on **http://localhost:5173** (dev mode)
2. Open **browser DevTools** (F12)
3. Go to **Console** tab
4. Look for errors
5. Share the error message

Or check the **Network** tab to see if `/api/get_job` is being called.
