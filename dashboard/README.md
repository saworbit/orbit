# Orbit Dashboard

Modern React dashboard for the Orbit Control Plane. Built with React 19, TypeScript, Vite, and Tailwind CSS.

## Features

- **Real-time Job Monitoring**: Auto-refreshing job list with progress tracking
- **Visual Pipeline Editor**: Drag-and-drop interface with React Flow
- **Professional File Browser**: Navigate and select files/folders with visual feedback
- **Quick Transfer**: Simplified copy/sync interface for common workflows
- **System Health Monitoring**: Real-time metrics and system status
- **User Administration**: Multi-user management with RBAC
- **Dark Mode Support**: Automatic theme switching based on system preferences

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
- **Tailwind CSS 4** - Utility-first styling
- **TanStack Query** - Data fetching and caching
- **React Flow** - Visual pipeline editor
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
│   │   ├── jobs/         # Job management
│   │   ├── pipelines/    # Visual pipeline editor
│   │   └── ui/           # Reusable UI primitives
│   ├── hooks/            # Custom React hooks
│   ├── lib/              # Utilities and API client
│   ├── App.tsx           # Main app component
│   └── main.tsx          # Entry point
├── public/               # Static assets
├── .prettierrc           # Prettier configuration
├── vitest.config.ts      # Vitest configuration
├── tailwind.config.js    # Tailwind configuration
└── package.json          # Dependencies and scripts
```

## API Integration

The dashboard connects to the Orbit backend at `http://localhost:3000/api`.

Key endpoints:
- `GET /api/files/list?path={path}` - File system navigation
- `POST /api/list_jobs` - Fetch all jobs
- `POST /api/create_job` - Create new transfer job
- `GET /api/stats/health` - System health metrics
- `GET /api/admin/users` - User management

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
