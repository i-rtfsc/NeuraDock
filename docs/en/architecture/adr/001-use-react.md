# ADR-001: Use React for Frontend

## Status
Accepted

## Context
NeuraDock needs a frontend framework for building the desktop application UI. The application requires:
- Complex state management for accounts and check-in operations
- Real-time updates from backend events
- Form handling for account configuration
- Internationalization support
- Good developer experience with TypeScript

## Decision
We chose **React 18** with the following ecosystem:
- **TypeScript 5**: Type safety
- **TanStack Query v5**: Server state management
- **Zustand**: Client state management
- **React Router v7**: Navigation
- **Tailwind CSS + Radix UI**: Styling and components
- **react-i18next**: Internationalization

## Consequences

### Positive
- Large ecosystem and community support
- Excellent TypeScript integration
- TanStack Query handles caching, refetching, and synchronization
- Radix UI provides accessible, unstyled components
- Easy integration with Tauri via tauri-specta

### Negative
- Requires careful management of re-renders
- Bundle size larger than alternatives like Solid or Svelte
- Need to manually optimize with useCallback/useMemo

### Alternatives Considered
- **Vue 3**: Good option but team more familiar with React
- **Solid**: Smaller bundle but smaller ecosystem
- **Svelte**: Compiler-based approach, less TypeScript maturity at time of decision

## Related
- [Architecture Overview](../architecture_overview.md)
- [ADR-003: DDD Architecture](./003-ddd-architecture.md)
