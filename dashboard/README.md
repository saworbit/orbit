# Orbit Dashboard v2.0

> ✅ **UI MIGRATION COMPLETE** - Production-ready dashboard with full API integration
> **Status**: v2.0.0-ui-migration (December 2025)
>
> ⚠️ **PRE-ALPHA SOFTWARE** - This dashboard is highly experimental and under active development.
> **NOT RECOMMENDED FOR PRODUCTION USE.** APIs and UI may change dramatically between versions.
> Use only for evaluation and testing with non-critical data.

Modern React dashboard for the Orbit Control Plane. Built with React 19, TypeScript, Vite, Tailwind CSS v4, and shadcn/ui.

## Features

### ✅ Production-Ready Features
- **Authentication System**: Complete login/logout flow with JWT token management and protected routes
- **Dashboard Screen**: Real-time KPI cards, network topology map, and live activity feed
- **Transfers Screen**: Job creation form, JobList, and **TeraCopy-style chunk map visualization**
- **Analytics Screen**: Real-time statistics with job success rates and performance metrics
- **Settings Screen**: Theme selector (Light/Dark/System), user management, and backend configuration
- **Dark Mode**: Complete theme system with localStorage persistence and system preference detection

### 🚧 Placeholder Features (UI Ready, API Pending)
- **Files Screen**: Professional file browser UI ready for `/api/list_dir` integration
- **Pipelines Screen**: Visual workflow editor placeholder ready for React Flow implementation
- **Analytics Charts**: Recharts placeholders for time-series visualizations

### Core Capabilities
- **Professional App Shell**: Sidebar navigation with user dropdown, logout, and responsive design
- **Real-time Updates**: 2-second auto-refresh for live job monitoring via TanStack Query
- **Chunk Map Visualization**: 100-cell grid showing transfer progress with color-coded states
- **Job Management**: Create, monitor, and inspect transfers with detailed progress tracking
- **User Interface**: 75+ shadcn/ui components with Radix UI primitives
- **Theme System**: Light/Dark/System modes with CSS variables
- **Type Safety**: Full TypeScript coverage with strict mode enabled

### UI/UX Highlights
- **Mobile-First Design**: Fully responsive from 320px to 4K displays
- **Real-time Search & Filtering**: Job search by ID, source, or destination with status filters
- **Visual Feedback**: Color-coded status badges (running/completed/failed), animated progress bars
- **Enhanced Empty States**: Helpful messaging with icons for better user guidance
- **Accessible Design**: Keyboard navigation, ARIA labels, and semantic HTML

## Quick Start

### Development

```bash
# Install dependencies
npm install

# Start dev server (with hot reload)
npm run dev

# Run all CI checks locally
npm run ci:check
```

### Production

```bash
# Build for production
npm run build

# Preview production build
npm run preview
```

## Available Scripts

| Script | Description |
|--------|-------------|
| `npm run dev` | Start Vite dev server with HMR (http://localhost:5173) |
| `npm run build` | Build for production (TypeScript + Vite, ~3.5s) |
| `npm run preview` | Preview production build (http://localhost:4173) |
| `npm run lint` | Run ESLint on all TypeScript files |
| `npm run format:check` | Check code formatting with Prettier |
| `npm run format:fix` | Auto-fix code formatting issues |
| `npm run typecheck` | Run TypeScript type checking (strict mode) |
| `npm run test` | Run tests with Vitest (watch mode) |
| `npm run ci:check` | **Run all checks before pushing** |

## Production Build

- **Build Time**: ~3.5 seconds
- **Bundle Size**: 340.6 KB (102.4 KB gzipped)
- **CSS Size**: 28.6 KB (5.7 KB gzipped)
- **Modules**: 1,806 transformed
- **Zero Errors**: TypeScript strict mode, ESLint, Prettier

## CI/CD Pipeline

The dashboard has a dedicated CI job (`dashboard-quality`) that runs on every push:

1. ✅ Formatting (Prettier)
2. ✅ Linting (ESLint)
3. ✅ Type checking (TypeScript strict mode)
4. ✅ Security audit (npm audit)
5. ✅ Unit tests (Vitest)
6. ✅ Production build verification

**Before pushing**, always run:
```bash
npm run ci:check
```

## Tech Stack

- **React 19** - UI library with latest features
- **TypeScript 5.9** - Type safety and better DX
- **Vite 7** - Lightning-fast build tool
- **Tailwind CSS 4** - Utility-first styling with tailwindcss-animate plugin
- **TanStack Query** - Data fetching and caching
- **@xyflow/react 12** - Visual pipeline editor (React Flow)
- **Lucide React** - Beautiful icon set
- **Axios** - HTTP client for API calls
- **Vitest** - Unit testing framework

## Project Structure

```
dashboard/
├── src/
│   ├── components/       # React components
│   │   ├── admin/        # User management UI
│   │   ├── dashboard/    # System health widgets
│   │   ├── files/        # File browser
│   │   ├── jobs/         # Job management & quick transfer
│   │   ├── layout/       # AppShell and navigation
│   │   ├── pipelines/    # Visual pipeline editor
│   │   └── ui/           # Reusable UI primitives
│   ├── hooks/            # Custom React hooks
│   ├── lib/              # Utilities and API client
│   ├── App.tsx           # Main app component with routing
│   └── main.tsx          # Entry point
├── public/               # Static assets
├── .prettierrc           # Prettier configuration
├── vitest.config.ts      # Vitest configuration
├── tailwind.config.js    # Tailwind configuration
└── package.json          # Dependencies and scripts
```

## API Integration

The dashboard connects to the Orbit backend at `http://localhost:8080/api` (configurable in `src/lib/api.ts`).

Key endpoints:
- `GET /api/files/list?path={path}` - File system navigation
- `POST /api/list_jobs` - Fetch all jobs (includes full job details)
- `POST /api/create_job` - Create new transfer job
- `POST /api/run_job` - Start a pending job
- `POST /api/cancel_job` - Cancel a running job
- `POST /api/delete_job` - Delete a job
- `GET /api/stats/health` - System health metrics (1s refresh in Mission Control)
- `GET /api/admin/users` - List all users
- `POST /api/admin/users` - Create new user
- `DELETE /api/admin/users/:id` - Delete user

**Note:** The JobDetail view currently uses mock data for UI demonstration (pre-alpha). A dedicated `GET /api/jobs/:id` endpoint is planned for future releases.

See the backend API documentation for full endpoint reference.

## Development Guidelines

### Code Style

- Use TypeScript for all new files
- Follow existing component patterns
- Use Tailwind CSS classes (avoid inline styles)
- Keep components small and focused
- Use React Query for data fetching

### Before Committing

1. Run `npm run ci:check` to validate all checks
2. Fix any ESLint warnings
3. Ensure TypeScript compiles without errors
4. Run `npm run format:fix` to auto-format code
5. Add tests for new features

### Component Guidelines

- Use functional components with hooks
- Prop types should be defined with TypeScript interfaces
- Extract reusable logic into custom hooks
- Use `lucide-react` for icons
- Follow the existing naming conventions

## Testing

```bash
# Run tests in watch mode
npm run test

# Run tests once (CI mode)
npm run test -- --run
```

Tests are written using Vitest. Place test files next to the components they test with `.test.tsx` extension.

## Troubleshooting

### Layout or Styling Issues
If you see layout constraints or the dashboard doesn't render edge-to-edge:
1. Ensure `App.css` only contains the comment about removed Vite styles
2. Verify `tailwindcss-animate` is in `package.json` dependencies
3. Run `npm install` to install the animation plugin
4. Restart your dev server (`npm run dev`)

### Port Already in Use
If port 5173 is already in use, Vite will automatically try the next available port.

### Backend Connection Issues
Ensure the Orbit backend is running on `http://localhost:3000`. Check the API client configuration in `src/lib/api.ts`.

### Type Errors
Run `npm run typecheck` to see all TypeScript errors. Make sure all dependencies are installed.

## Contributing

1. Create a feature branch from `main`
2. Make your changes following the guidelines above
3. Run `npm run ci:check` and fix any issues
4. Submit a pull request with a clear description

## License

Apache 2.0 - See LICENSE file in the root directory.
