# Full UI Replacement: Figma Mockup → Orbit Production

## 🎯 Goal
Replace current Orbit dashboard entirely with Figma mockup, integrating all Orbit APIs.

## 📦 Migration Checklist

### Phase 1: Preparation (30 min)
- [x] Backup current dashboard
- [x] Analyze Figma component structure
- [x] Identify API integration points
- [ ] Create migration branch

### Phase 2: Core Integration (2-3 hours)
- [ ] Copy Figma mockup structure to main dashboard
- [ ] Install any missing dependencies
- [ ] Integrate Orbit API hooks:
  - [ ] `useJobs` → Dashboard KPIs, Transfers screen
  - [ ] `useBackends` → Settings, Network map
  - [ ] `useAuth` → Login/logout
  - [ ] `useRunJob`, `useCancelJob`, `useDeleteJob`
- [ ] Wire up authentication flow
- [ ] Update routing structure

### Phase 3: Screen-by-Screen Adaptation (4-6 hours)

#### Dashboard (Command Center)
- [ ] KPICards: Connect to real job stats
  - Active Jobs: `jobs.filter(j => j.status === 'running').length`
  - Total Transferred: Sum of `completed_chunks`
  - System Load: From system metrics
  - Active Backends: `backends.filter(b => b.status === 'online').length`
- [ ] NetworkMap: Show real source → destination topology
  - Parse job sources/destinations
  - Group by backend type
  - Live status indicators
- [ ] ActivityFeed: Real-time reactor events
  - Subscribe to WebSocket `/ws/events`
  - Show job creation, completion, errors
  - Auto-scroll to latest

#### Transfers Screen
- [ ] **Integrate JobDetail chunk map!** (MUST PRESERVE)
  - Keep your production-ready chunk visualization
  - Replace static transfer panel with chunk map view
  - Click job → shows chunk map grid
- [ ] TransferPanel: Connect to `/api/create_job`
  - LocationSelector → real file browser
  - Advanced options → compress, verify, parallel workers
- [ ] StatusLog: Live job progress
  - Poll `/api/get_job` every second
  - Show percentage, chunks, errors

#### Files Screen
- [ ] File browser integration
  - Connect to `/api/list_dir` endpoint
  - Breadcrumb navigation
  - File/folder icons
  - Select source/destination
- [ ] Actions: Copy, move, delete
  - Create job from file selection

#### Pipelines Screen
- [ ] Keep React Flow editor? Or use Figma design?
  - **Decision needed**: Current editor is functional
  - Figma design looks simpler
  - **Recommendation**: Merge both (React Flow + Figma styling)

#### Analytics Screen
- [ ] Charts integration
  - Job completion rate over time
  - Throughput graphs
  - Error trends
  - Backend utilization
- [ ] Use Chart.js or Recharts
  - Data from job history

#### Settings Screen
- [ ] Backend configuration
  - CRUD for S3, SMB, SSH backends
  - Connection testing
- [ ] User management (if admin)
  - Integrate current UserList
- [ ] System preferences
  - Theme toggle (keep dark mode option!)
  - Notification settings

### Phase 4: Theme & Polish (2 hours)
- [ ] Unify color palette
  - Keep Figma's Slate colors for light mode
  - Add dark mode support (toggle in settings)
  - CSS variables for both themes
- [ ] Responsive design verification
  - Test on mobile, tablet, desktop
- [ ] Loading states
- [ ] Error handling
- [ ] Empty states

### Phase 5: Testing & Deployment (2 hours)
- [ ] E2E test all screens
- [ ] Verify API integration
- [ ] Test authentication flow
- [ ] Build production bundle
- [ ] Update launch scripts

---

## 🔑 Critical Integrations

### 1. Preserve JobDetail Chunk Map
**Location**: Transfers screen

```typescript
// dashboard/src/components/screens/Transfers.tsx
import { JobDetail } from '../jobs/JobDetail'; // Your chunk map!

export function Transfers() {
  const [selectedJobId, setSelectedJobId] = useState<number | null>(null);

  if (selectedJobId) {
    return <JobDetail jobId={selectedJobId} onBack={() => setSelectedJobId(null)} />;
  }

  return (
    <div>
      <TransferPanel />
      <JobList onSelectJob={setSelectedJobId} />
    </div>
  );
}
```

### 2. Real-time Updates
**WebSocket Integration**

```typescript
// dashboard/src/hooks/useReactorEvents.ts
export function useReactorEvents() {
  useEffect(() => {
    const ws = new WebSocket('ws://localhost:8080/ws/events');
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      // Update activity feed
      // Update job status
    };
    return () => ws.close();
  }, []);
}
```

### 3. Authentication
**Login Flow**

```typescript
// dashboard/src/components/screens/Login.tsx (new)
export function Login({ onLogin }: { onLogin: () => void }) {
  const handleSubmit = async (username: string, password: string) => {
    const response = await api.post('/auth/login', { username, password });
    if (response.ok) {
      onLogin();
    }
  };
}
```

---

## 📁 File Structure (After Migration)

```
dashboard/src/
├── App.tsx                    # Figma structure
├── main.tsx                   # Entry point
├── components/
│   ├── Header.tsx             # Figma
│   ├── Sidebar.tsx            # Figma
│   ├── Footer.tsx             # Figma
│   ├── screens/
│   │   ├── Dashboard.tsx      # Figma + API
│   │   ├── Transfers.tsx      # Figma + JobDetail chunk map!
│   │   ├── Files.tsx          # Figma + API
│   │   ├── Pipelines.tsx      # Hybrid (React Flow + Figma)
│   │   ├── Analytics.tsx      # Figma + Charts
│   │   └── Settings.tsx       # Figma + UserList
│   ├── dashboard/
│   │   ├── KPICards.tsx       # Figma → API
│   │   ├── NetworkMap.tsx     # Figma → API
│   │   └── ActivityFeed.tsx   # Figma → WebSocket
│   ├── jobs/
│   │   ├── JobDetail.tsx      # PRESERVED! (Your chunk map)
│   │   └── JobList.tsx        # Existing
│   └── auth/
│       └── Login.tsx          # New
├── hooks/
│   ├── useJobs.ts             # Existing
│   ├── useBackends.ts         # Existing
│   ├── useAuth.ts             # New
│   └── useReactorEvents.ts    # New (WebSocket)
└── lib/
    └── api.ts                 # Existing
```

---

## 🎨 Theme Strategy

### Light Mode (Default - Figma)
```css
:root {
  --background: #f8fafc; /* slate-50 */
  --foreground: #0f172a; /* slate-900 */
  --primary: #2563eb;    /* blue-600 */
  --accent: #f1f5f9;     /* slate-100 */
}
```

### Dark Mode (Optional Toggle)
```css
[data-theme="dark"] {
  --background: hsl(222.2 84% 4.9%);
  --foreground: hsl(210 40% 98%);
  --primary: hsl(217.2 91.2% 59.8%);
  --accent: hsl(217.2 32.6% 17.5%);
}
```

**Theme toggle** in Settings screen.

---

## ⏱️ Timeline

| Phase | Duration | Status |
|-------|----------|--------|
| Phase 1: Preparation | 30 min | ⏳ In Progress |
| Phase 2: Core Integration | 2-3 hours | 🔜 Next |
| Phase 3: Screens (Dashboard) | 1 hour | 🔜 Pending |
| Phase 3: Screens (Transfers + Chunk Map) | 1.5 hours | 🔜 Pending |
| Phase 3: Screens (Files) | 1 hour | 🔜 Pending |
| Phase 3: Screens (Pipelines) | 1 hour | 🔜 Pending |
| Phase 3: Screens (Analytics) | 1.5 hours | 🔜 Pending |
| Phase 3: Screens (Settings) | 1 hour | 🔜 Pending |
| Phase 4: Theme & Polish | 2 hours | 🔜 Pending |
| Phase 5: Testing | 2 hours | 🔜 Pending |
| **Total** | **~12-14 hours** | **🚀 Let's Go!** |

---

## 🚨 Important Decisions

### 1. Pipelines Editor
**Current**: React Flow (fully functional)
**Figma**: Simpler design

**Options**:
- A. Keep React Flow, apply Figma styling
- B. Replace with Figma design (lose Flow editor)
- C. Hybrid: Figma for simple transfers, React Flow for advanced pipelines

**Recommendation**: **Option A** (keep functionality, update styling)

### 2. Dark Mode
**Current**: Full dark mode support
**Figma**: Light mode only

**Options**:
- A. Light mode only (follow Figma exactly)
- B. Add dark mode toggle (more work, better UX)

**Recommendation**: **Option B** (add dark mode, users love it)

### 3. JobDetail Chunk Map Placement
**Where to show the chunk map?**

**Option A**: Transfers screen (click job → chunk map)
**Option B**: Separate "Job Details" screen
**Option C**: Modal overlay on any screen

**Recommendation**: **Option A** (natural fit in Transfers)

---

## 🎯 Success Criteria

Migration is complete when:

✅ All 6 screens functional (Dashboard, Transfers, Files, Pipelines, Analytics, Settings)
✅ JobDetail chunk map preserved and working
✅ Authentication flow working
✅ Real API data displayed (no mock data)
✅ WebSocket real-time updates working
✅ Dark mode toggle available
✅ Production build successful
✅ All tests passing

---

## 🚀 Ready to Start?

**I'll begin with Phase 1**: Backup current dashboard and set up structure.

Estimated completion: **Tonight** (if we work through it) or **tomorrow** (with breaks).

**Let's build something amazing!** 🎨
