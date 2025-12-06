# UI Migration Plan: Figma Mockup → Orbit Production

## 🎨 Current State Analysis

### Figma Mockup (`dashboard/ui_mockup/`)
**Screens:**
- ✅ Dashboard (Command Center with KPI cards, network map, activity feed)
- ✅ Transfers (transfer panel, advanced transfer controls)
- ✅ Files (file browser/explorer)
- ✅ Pipelines (workflow editor)
- ✅ Analytics (metrics and charts)
- ✅ Settings (configuration)

**Components:**
- Header (with branding, notifications)
- Sidebar (collapsible navigation)
- Footer
- LocationSelector
- StatusLog
- TransferPanel
- ControlBar

**Design System:**
- Slate color palette (professional blue-gray)
- Tailwind CSS
- Modern card-based layouts
- Network topology visualization

### Current Orbit UI (`dashboard/src/`)
**Screens:**
- ✅ Dashboard (Overview with system health)
- ✅ Jobs (JobList + JobDetail with chunk map)
- ✅ Quick Transfer
- ✅ Pipelines (React Flow editor)
- ✅ Admin (UserList)

**Components:**
- AppShell (sidebar navigation)
- JobDetail (with visual chunk map - NEW!)
- JobList
- QuickTransfer
- PipelineEditor
- UserList

**Design System:**
- Dark theme with theme toggle
- Shadcn/ui components
- TeraCopy-inspired chunk visualization

---

## 🎯 Migration Strategy

### Option 1: **Full Replacement** (Big Bang)
Replace entire Orbit UI with Figma mockup and adapt to Orbit's API.

**Pros:**
- Professional, cohesive design
- Clean slate
- Modern UX patterns

**Cons:**
- Loses JobDetail chunk map (your favorite feature!)
- Requires complete API integration
- Testing overhead

**Effort:** 2-3 weeks

---

### Option 2: **Cherry-Pick Components** (Recommended)
Take the best components from Figma mockup and integrate into current Orbit.

**What to Keep from Current Orbit:**
- ✅ **JobDetail with chunk map** (your production-ready feature!)
- ✅ **AppShell** (works well with dark theme)
- ✅ **PipelineEditor** (React Flow integration complete)
- ✅ **UserList** (functional admin panel)

**What to Take from Figma Mockup:**
- ✅ **Dashboard/KPICards** → Replace current Overview
- ✅ **NetworkMap** → Add visual topology view
- ✅ **ActivityFeed** → Add real-time event stream
- ✅ **TransferPanel** → Enhance QuickTransfer
- ✅ **Files browser** → Add file explorer (currently missing!)
- ✅ **Header/Footer** → Optional branding upgrade

**Pros:**
- Keep your chunk map!
- Best of both worlds
- Incremental migration (lower risk)

**Cons:**
- Need to unify design system (merge Slate + dark theme)
- Component naming conflicts

**Effort:** 1 week per screen

---

### Option 3: **Hybrid Themes** (Power User Mode)
Keep both UIs and let users toggle between them!

**Light Mode** = Figma mockup (professional, client-facing)
**Dark Mode** = Current Orbit (power user, chunk map focus)

**Pros:**
- Maximum flexibility
- Different UX for different use cases
- No features lost

**Cons:**
- Maintenance overhead (2 UIs)
- Design inconsistency

**Effort:** 2 weeks + ongoing dual maintenance

---

## 🚀 Recommended: Option 2 (Cherry-Pick)

### Phase 1: Visual Enhancements (Week 1)
1. **Integrate KPICards** into Dashboard
   - Active Jobs, Total Transferred, System Load, Backends
   - Replace SystemHealth component

2. **Add NetworkMap** visualization
   - Show source → destination topology
   - Visual backend status

3. **Upgrade Header**
   - Add Figma header with Orbit branding
   - Keep theme toggle

### Phase 2: Add Missing Features (Week 2)
4. **Add Files browser** (from Figma mockup)
   - File explorer for browsing source/destination
   - Currently missing in Orbit!

5. **Enhance TransferPanel**
   - Use Figma's LocationSelector
   - Better UX than current QuickTransfer

### Phase 3: Polish (Week 3)
6. **Add ActivityFeed** to Dashboard
   - Real-time job events
   - Integrates with reactor logs

7. **Unified color palette**
   - Merge Slate (light) + current dark theme
   - CSS variable system

---

## 📋 Step-by-Step: Integrate KPICards (Example)

### 1. Copy Component
```bash
cp dashboard/ui_mockup/src/components/dashboard/KPICards.tsx \
   dashboard/src/components/dashboard/KPICards.tsx
```

### 2. Adapt to Orbit API
```typescript
// Before (Figma mockup - fake data)
const kpis = [
  { title: 'Active Jobs', value: '12', change: '+3' },
  // ...
];

// After (Orbit - real API)
import { useJobs } from '../../hooks/useJobs';

export function KPICards() {
  const { data: jobs } = useJobs();
  const activeJobs = jobs?.filter(j => j.status === 'running').length || 0;

  return (
    <div className="grid grid-cols-4 gap-4">
      <Card>
        <h3>Active Jobs</h3>
        <div className="text-3xl font-bold">{activeJobs}</div>
      </Card>
      {/* ... */}
    </div>
  );
}
```

### 3. Wire into App
```typescript
// dashboard/src/components/dashboard/Overview.tsx
import { KPICards } from './KPICards';
import { NetworkMap } from './NetworkMap';
import { ActivityFeed } from './ActivityFeed';

export default function Overview() {
  return (
    <div className="space-y-6">
      <KPICards />
      <NetworkMap />
      <ActivityFeed />
    </div>
  );
}
```

---

## 🎨 Design System Unification

### Colors
Merge both palettes:

```css
/* Figma Mockup (Light) */
--slate-50: #f8fafc;
--slate-900: #0f172a;
--blue-600: #2563eb;

/* Current Orbit (Dark) */
--background: 222.2 84% 4.9%;
--foreground: 210 40% 98%;
--primary: 217.2 91.2% 59.8%;

/* Unified Palette */
:root {
  --slate-bg: #f8fafc;
  --orbit-dark: hsl(222.2 84% 4.9%);
}

[data-theme="light"] { /* Use Figma colors */ }
[data-theme="dark"] { /* Use Orbit colors */ }
```

---

## 🧪 Testing the Mockup

The mockup is now running! Check it out:

```bash
# Mockup preview (Figma design)
http://localhost:5174  # Different port to avoid conflict

# Current Orbit
http://localhost:5173
```

**Compare them side-by-side!**

---

## 💡 My Recommendation

**Start with the Dashboard screen:**

1. Keep your **JobDetail chunk map** (it's production-ready and unique!)
2. Replace **Overview** with Figma's **KPICards + NetworkMap**
3. Add **ActivityFeed** for real-time logs
4. **Keep everything else** as-is for now

This gives you:
- ✅ Professional dashboard (from Figma)
- ✅ Power-user job details (your chunk map)
- ✅ Minimal disruption
- ✅ Quick win (2-3 days work)

---

## 🤔 What Do You Think?

**Which approach do you prefer?**

A. **Full replacement** - Use Figma mockup entirely
B. **Cherry-pick** (recommended) - Best of both worlds
C. **Keep current Orbit** - Just polish what you have

**Or specific components you want?**
- NetworkMap visualization?
- Files browser?
- KPI cards?
- Activity feed?

Let me know and I'll start the integration!
