# Orbit Control Plane - Dashboard Design Philosophy

> ⚠️ **PRE-ALPHA DESIGN DOCUMENT**
> This design is experimental and subject to significant changes.
> Current implementation may not fully reflect this vision.

---

## 1. Design Philosophy & Theme

### Project Identity
**Name:** Orbit Control Plane (v2.2)

**Status:** 🔴 Pre-Alpha - Highly experimental, breaking changes expected

**Mission:** Transform Orbit from a CLI-focused tool into a modern Enterprise Data Platform with emphasis on visibility (Dashboards) and ease of use (Wizards/Visual Editors).

### Visual Style
- **Modern, Clean, and Data-Dense**: Optimized for monitoring complex data transfer operations
- **Default Theme**: Dark Mode ("Orbit Dark") - Optimized for long monitoring sessions with reduced eye strain
- **Alternative**: High-contrast Light Mode for accessibility and preference
- **Color Philosophy**: Semantic color usage for instant status recognition (Green=Success, Blue=Processing, Yellow=Warning, Red=Error)

### Layout Strategy
**Top-Level Navigation Bar** for global context, maximizing screen real estate for complex views like:
- Pipeline Editor (React Flow canvas)
- Job Lists (dense tabular data)
- System Health Monitoring

**Note:** Current implementation uses **Sidebar Navigation** - this will be evaluated and may revert to top navigation based on user feedback and space optimization needs.

### Tech Stack
- **React 19** - Latest features with concurrent rendering
- **TypeScript 5.9** - Type safety throughout
- **Vite 7** - Lightning-fast HMR and builds
- **Tailwind CSS 4** - Utility-first styling with design tokens
- **Lucide Icons** - Consistent, modern iconography
- **React Flow (@xyflow/react 12)** - Visual pipeline editor
- **TanStack Query** - Intelligent data fetching and caching

---

## 2. Layout Structure

The application uses a **Single Page Application (SPA)** architecture with:
- Persistent navigation (currently sidebar, may move to top bar)
- Dynamic main content area with route-based views
- Shared state management via React Context and TanStack Query

### A. Global Navigation

**Current Implementation (Sidebar):**
- **Left Section (Branding)**:
  - Logo: 🛸 Icon + "Orbit Control Plane" text (Bold, text-lg)
  - Subtitle: "Control Plane" (text-[10px], uppercase, tracking-widest)

- **Navigation Items**:
  - **Overview** (Dashboard) - System health + recent activity
  - **Job Manager** - Real-time monitoring list
  - **Pipelines** - Visual workflow editor + Quick Transfer
  - **Administration** - User management + system settings

- **Bottom Section**:
  - Theme Toggle (Sun/Moon icon)
  - User Profile (Gradient avatar + username)
  - Logout button

**Future Consideration:** Top navigation bar for more horizontal space in editor views.

### B. Pre-Alpha Warning Banner
**Permanent fixture** across all views:
- Gradient background (Yellow → Orange → Red) for high visibility
- Warning icon and clear messaging
- Persistent reminder of experimental status
- Non-dismissible to ensure awareness

---

## 3. Core Views & Components

### A. Mission Control Dashboard (`/dashboard`)
**Purpose:** Real-time telemetry and flight status with live data visualization

**Key Innovation: Client-Side Data Buffering**
- Backend provides point-in-time stats via `/stats/health` endpoint
- Frontend maintains 30-point rolling history buffer
- Creates "live" feel with smooth animations despite stateless backend
- Updates every 1 second with React Query aggressive polling

**Components:**
- **Live Status Indicator**:
  - Pulsing green dot animation with double-ring effect
  - "Live Stream Active" badge
  - Visual confirmation of real-time monitoring

- **Metric Cards** (4-column grid):
  - **Active Jobs**: Yellow Zap icon, shows running count with trend
  - **Throughput**: Blue Activity icon, displays MB/s with percentage change
  - **System Load**: Purple Server icon, percentage with stability indicator
  - **Storage Health**: Green HardDrive icon, status text with health state
  - Each card includes trend indicator (ArrowUpRight/ArrowDownRight)
  - Hover effects with border color transitions

- **Network Throughput Graph**:
  - SVG area chart with gradient fill and stroke line
  - 200px height canvas showing 30 data points
  - Real-time updates as new data arrives
  - Current value displayed prominently (large font, blue accent)
  - Statistics bar: Peak, Average, Total Transferred
  - Gradient background from `stopColor` with opacity fade

- **Capacity Planning Panel**:
  - CSS-based donut chart (72% usage visualization)
  - Rotating border technique for visual percentage
  - Used Space (Primary color) vs. Available (Muted)
  - Breakdown: 84.2 TB used, 32.8 TB available

**Design Goal:** Cockpit-style monitoring with emphasis on visual data density and "live" responsiveness.

### B. Jobs Dashboard (`/jobs`)
**Purpose:** Central monitoring hub for all transfers with drill-down capability

**Navigation Flow:**
- Job List View (default) → Click job row → Job Detail View
- Job Detail → Click "Back to Job List" → Returns to filtered list
- State management via React `useState` for `selectedJobId`

**UI Components (List View):**
- **Status Indicators**:
  - Color-coded badges with dark mode support:
    - 🟢 Green = Completed
    - 🔵 Blue = Running
    - 🟡 Yellow = Pending
    - 🔴 Red = Failed
    - ⚪ Gray = Cancelled

- **Search & Filter Bar**:
  - Real-time search by Job ID, source path, or destination path
  - Status dropdown filter
  - Manual refresh button

- **Progress Visualization**:
  - Animated progress bars
  - Chunk completion ratio (e.g., "1,245/2,000 chunks")
  - Percentage complete with ETA

- **Dense Table View**:
  - Job ID (monospace font)
  - Source → Destination (with truncation)
  - Status badge
  - Action buttons (Run, Cancel, Delete)
  - **Clickable rows** with hover effect (`cursor-pointer`)

- **Auto-Refresh**: React Query background polling (2s interval) without full page reloads

**Empty State:** Helpful icon + message encouraging first job creation

#### B.1. Job Detail View (Deep Dive)
**Purpose:** Visual inspection of individual job with chunk-level granularity

**Layout Structure:**
- Breadcrumb navigation: "Job List" → "Job #N"
- Back button with ChevronLeft icon
- Three-column grid layout (responsive to single column on mobile)

**Key Components:**

**1. Visual Chunk Map**
- **Innovation**: Dense 100-cell grid representing job progress
- **Rendering Strategy**:
  - Total chunks > 100: Proportional sampling (e.g., 2000 chunks → each cell = 20 chunks)
  - Total chunks ≤ 100: 1:1 mapping with remaining cells as "pending"
- **Visual States**:
  - Green cells with glow (`shadow-[0_0_8px_rgba(34,197,94,0.6)]`) = Completed
  - Red cells with glow (`shadow-[0_0_8px_rgba(239,68,68,0.6)]`) = Failed
  - Gray muted cells = Pending
- **Grid Configuration**: `grid-cols-20` (20×5 arrangement)
- **Cell Size**: Responsive (`h-1.5 w-1.5` on mobile, `h-2 w-2` on desktop)
- **Animation**: `transition-all duration-500` for smooth state changes

**2. Performance Metrics Cards**
- **Throughput**: Activity icon, MB/s display, green trend indicator
- **Chunk Statistics**: Package icon, completion ratio
- **Transfer Speed**: Gauge icon, real-time speed with comparison to average

**3. Event Stream**
- Chronological list of job lifecycle events
- Timestamp with monospace formatting
- Status icons (CheckCircle=success, AlertCircle=error, Info=info)
- Auto-scroll to latest event
- Color-coded text by event type

**4. Configuration Panel**
- Source/Destination paths (monospace, truncated)
- Mode badge (Copy/Sync)
- Boolean flags: Compression, Verification (CheckCircle/XCircle icons)
- Workers count, Chunk size, Retry attempts

**Data Fetching:**
- **Current (Pre-Alpha)**: Uses mock data for UI demonstration
- **Planned**: Dedicated `GET /api/jobs/:id` endpoint for individual job details
- **Alternative**: Can use existing `GET /api/jobs` and filter client-side by ID
- **Future**: 2-second polling for running jobs, manual refresh, error boundaries

**Design Goal:** Provide operator-level visibility into job internals for debugging and monitoring.

### C. Job Creation Wizard (`/create`)
**Purpose:** Simplified flow for standard file transfer tasks

**Note:** Currently deprecated in favor of Quick Transfer - may be removed or reimagined.

**Proposed UI Flow:**
1. **Source Selection**: Visual file browser for path selection
2. **Destination Selection**: Similar browser for target path
3. **Configuration Panel**:
   - Compression toggle
   - Verification toggle
   - Parallel workers slider (1-16)
4. **Validation**: Real-time path checks and permission verification
5. **Summary & Launch**: Review and confirm

### D. Pipeline Editor (`/pipelines`)
**Purpose:** Visual workflow design and quick transfers

**Two Modes (Tab-based toggle):**

#### 1. Quick Transfer Mode (Default)
- **Simplified Interface**: Source → Destination card layout
- **Visual Flow**:
  - Blue-coded source selector (left)
  - Orange-coded destination selector (right)
  - Animated arrow connector showing data flow
- **Mode Toggle**: Copy vs. Sync (with icons)
- **Focus**: Immediate, one-off transfers without complexity

#### 2. Advanced Visual Editor (React Flow)
- **Infinite Canvas**: Grid-based draggable workspace
- **Node Types**:
  - **Source Nodes** (Input): S3, Local, SMB, etc.
  - **Transform Nodes** (Default): Compress, Encrypt, Filter, etc.
  - **Destination Nodes** (Output): S3, Local, Cloud, etc.
- **Edge System**: Connectable lines defining data flow
- **Toolbar**:
  - "Add Source" (Database icon)
  - "Add Transform" (Zap icon)
  - "Add Destination" (Cloud icon)
  - Node/edge counter
- **Theme Integration**: Canvas background and controls match app theme

**Future:** Pipeline save/load, templates, scheduling integration

### E. Administration (`/admin`)
**Purpose:** System and user management

**UI Components:**

#### User Management
- **Statistics Cards**:
  - Total Users (Blue icon)
  - Administrators (Purple icon)
  - Operators (Green icon)

- **User Table**:
  - Gradient avatars with initials
  - Username + role badge
  - Creation date
  - Delete action (with confirmation)

- **Add User Form**:
  - Username, Password, Role fields
  - Validation and error handling
  - Theme-aware styling

#### System Health (Integration)
- CPU usage widget
- Memory utilization
- Active thread counts
- Storage metrics

### F. File Browser Overlay
**Purpose:** Reusable component for path selection across wizards and editors

**Features:**
- **Breadcrumb Navigation**: Current path display with clickable segments
- **Icon System**:
  - 📁 Blue folders
  - 📄 Gray files
  - ✓ CheckCircle for selected items
- **Up Navigation**: Arrow button for parent directory
- **Selection States**:
  - Visual highlight (blue background in dark mode)
  - "Select Current Folder" button
  - Individual file/folder selection
- **Loading & Error States**: Spinner and error messages
- **RESTful API**: `GET /api/files/list?path={encoded_path}`

---

## 4. Visual Component System (Tailwind)

### Design Token Strategy
Utilizing **semantic naming conventions** (shadcn/ui patterns):

#### Core Colors
- `bg-background` / `text-foreground` - Main app canvas
- `bg-card` - Elevated surfaces (lists, widgets, modals)
- `text-muted-foreground` - Secondary text (descriptions, timestamps)
- `bg-primary` - Call-to-action buttons (Create Job, Save Pipeline)
- `bg-accent` - Hover states and secondary interactions
- `border-border` - Dividers and card edges

#### Status Colors (with Dark Mode variants)
```css
/* Success */
bg-green-100 text-green-800 (light)
dark:bg-green-900/30 dark:text-green-400 (dark)

/* Info/Processing */
bg-blue-100 text-blue-800 (light)
dark:bg-blue-900/30 dark:text-blue-400 (dark)

/* Warning */
bg-yellow-100 text-yellow-800 (light)
dark:bg-yellow-900/30 dark:text-yellow-400 (dark)

/* Error */
bg-red-100 text-red-800 (light)
dark:bg-red-900/30 dark:text-red-400 (dark)
```

### Typography
- **Font Family**: Sans-serif, optimized for technical data readability
- **Monospace**: Used for paths, hashes, logs, job IDs
- **Hierarchy**:
  - `text-3xl font-bold` - Page headers
  - `text-xl font-semibold` - Section headers
  - `text-sm` - Body text
  - `text-xs` - Helper text, timestamps
  - `text-[10px]` - Labels, subtitles

### Spacing & Layout
- **Container**: `max-w-7xl mx-auto` - Centered content with breathing room
- **Card Padding**: `p-4` to `p-6` depending on content density
- **Grid Systems**: `grid-cols-1 md:grid-cols-2 lg:grid-cols-4` for responsive layouts

---

## 5. Interaction Design

### Feedback Mechanisms

#### Loading States
- **Skeletons**: Animated placeholders during initial data fetch (React Suspense)
- **Spinners**: For button actions and file browser loading
- **Progress Bars**: For transfer operations with percentage + chunk count
- **Sparklines**: Trend visualization for system metrics

#### Notifications
- **Toast Messages** (Future Implementation):
  - "Job Started" (Green)
  - "Transfer Failed" (Red)
  - "Pipeline Saved" (Blue)
- **Inline Alerts**: Success/error states within forms

#### Transitions
- **Color Transitions**: `transition-colors` on hover states
- **Theme Switching**: Smooth fade between dark/light modes
- **Menu Animations**: `transition-transform duration-300` for mobile drawer
- **Page Transitions**: `animate-in fade-in slide-in-from-bottom-4` for content

### Accessibility
- **Keyboard Navigation**: Tab order and focus management
- **ARIA Labels**: Screen reader support for icons and actions
- **Color Contrast**: WCAG AA compliance in both themes
- **Touch Targets**: Minimum 44x44px for mobile interactions

---

## 6. Data Flow & State Management

### React Query Patterns
- **Job List**: 2-second polling with automatic cache invalidation
- **System Health**: 2-second polling for live metrics
- **User Management**: Mutation hooks for CRUD operations with optimistic updates
- **File Browser**: On-demand fetching per directory navigation

### Error Handling
- **Network Errors**: Retry logic with exponential backoff
- **API Errors**: User-friendly messages with technical details in console
- **Validation Errors**: Inline field-level feedback

---

## 7. Responsive Design Strategy

### Breakpoints
- **Mobile**: 320px - 767px (Hamburger menu, stacked layouts)
- **Tablet**: 768px - 1023px (Sidebar visible, 2-column grids)
- **Desktop**: 1024px+ (Full layout, 4-column grids, expanded tables)

### Mobile Optimizations
- **Drawer Menu**: Slide-in sidebar with backdrop overlay
- **Touch Targets**: Enlarged buttons and hit areas
- **Horizontal Scroll**: Tables scroll horizontally on small screens
- **Compact Cards**: Reduced padding and simplified layouts

---

## 8. Future Considerations

### Planned Enhancements
- **WebSocket Integration**: Real-time progress updates (<50ms latency)
- **Pipeline Templates**: Pre-built workflows for common scenarios
- **Advanced Filtering**: Saved filter presets and complex queries
- **Export Capabilities**: Download job reports and metrics
- **Audit Logging UI**: Visual timeline of all system actions
- **Multi-language Support**: i18n infrastructure

### Performance Goals
- **Initial Load**: <2s on 4G connection
- **Time to Interactive**: <3s
- **Bundle Size**: Target <200KB gzipped (currently 163KB ✓)

---

## 9. Development Principles

### Code Quality
- **TypeScript Strict Mode**: Zero `any` types, full type coverage
- **ESLint Zero Warnings**: Clean linting on every commit
- **Prettier Formatting**: Consistent code style
- **Component Testing**: Vitest unit tests for critical paths
- **CI/CD Validation**: All checks pass before merge

### Component Patterns
- **Composition over Inheritance**: Small, focused components
- **Custom Hooks**: Extract reusable stateful logic
- **Prop Interfaces**: Clear TypeScript contracts
- **Error Boundaries**: Graceful failure handling

---

## 10. Pre-Alpha Caveats

### Known Limitations
- ⚠️ **Breaking Changes Expected**: APIs and UI will change significantly
- ⚠️ **Incomplete Features**: Some views are placeholders or minimal implementations
- ⚠️ **Limited Testing**: User acceptance testing is ongoing
- ⚠️ **Performance**: Not optimized for large-scale deployments (1000+ jobs)
- ⚠️ **Browser Support**: Tested primarily on Chrome/Firefox latest versions

### Not Recommended For
- ❌ Production environments with mission-critical data
- ❌ Systems requiring regulatory compliance auditing
- ❌ High-security environments without thorough security review
- ❌ Large teams without acceptance of breaking changes

### Recommended For
- ✅ Internal testing and evaluation
- ✅ Development environments
- ✅ Proof-of-concept demonstrations
- ✅ Feedback and feature validation

---

## Conclusion

This design philosophy document reflects the **vision** for Orbit Control Plane v2.2. The current implementation is a **pre-alpha prototype** that demonstrates these concepts but is **not production-ready**.

We prioritize **rapid iteration and user feedback** over stability at this stage. As the platform matures, this document will evolve to reflect lessons learned and refined design decisions.

**Feedback Welcome:** Please open GitHub issues with UX suggestions, design critiques, or usability concerns.

---

**Last Updated:** 2025-12-04
**Document Status:** 🔴 Pre-Alpha - Subject to major revisions
**Author:** Orbit Development Team (with Claude Code assistance)
