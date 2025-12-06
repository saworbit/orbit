# Orbit Dashboard

> ⚠️ **PRE-ALPHA SOFTWARE** - This dashboard is highly experimental and under active development.
> **NOT RECOMMENDED FOR PRODUCTION USE.** APIs and UI may change dramatically between versions.
> Use only for evaluation and testing with non-critical data.

Modern React dashboard for the Orbit Control Plane. Built with React 19, TypeScript, Vite, and Tailwind CSS.

## Features

### Core Capabilities
- **Cockpit-Style App Shell**: Professional sidebar navigation with live status indicators and responsive mobile drawer menu
- **Mission Control Dashboard**: Real-time telemetry with live network throughput graphs, metric cards, and capacity planning
- **Deep-Dive Job Details**: Visual chunk map with 100-cell grid, event stream, and comprehensive performance metrics
- **Real-time Job Monitoring**: Auto-refreshing job list with search, filters, progress tracking, and click-to-expand details
- **Visual Pipeline Editor**: Drag-and-drop interface with React Flow v12
- **Professional File Browser**: Navigate and select files/folders with visual feedback
- **Quick Transfer**: Simplified copy/sync interface with visual source→destination flow
- **System Health Monitoring**: Live metrics with SVG sparkline trend visualizations
- **User Administration**: Multi-user management with RBAC, statistics, and delete functionality
- **Dark Mode Support**: Seamless theme switching with consistent styling across all components

### UI/UX Highlights
- **Mobile-First Design**: Fully responsive from 320px to 4K displays
- **Search & Filtering**: Real-time job search by ID, source, or destination
- **Visual Feedback**: Color-coded status badges, gradient avatars, and animated transitions
- **Enhanced Empty States**: Helpful messaging with icons for better user guidance
- **Keyboard Navigation**: Accessible interface with proper focus management

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
| `npm run dev` | Start Vite dev server with HMR |
| `npm run build` | Build for production (TypeScript + Vite) |
| `npm run preview` | Preview production build locally |
| `npm run lint` | Run ESLint on all TypeScript files |
| `npm run format:check` | Check code formatting with Prettier |
| `npm run format:fix` | Auto-fix code formatting issues |
| `npm run typecheck` | Run TypeScript type checking |
| `npm run test` | Run tests with Vitest (watch mode) |
| `npm run ci:check` | **Run all checks before pushing** |

## CI/CD Pipeline

The dashboard has a dedicated CI job (`dashboard-quality`) that runs on every push:

1. ✅ Formatting (Prettier)
2. ✅ Linting (ESLint)
3. ✅ Type checking (TypeScript)
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
