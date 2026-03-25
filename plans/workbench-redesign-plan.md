# Desktop-First Workbench Redesign - Architectural Plan

## Shared Layout Architecture

### Core Layout Components
The redesign will use and enhance existing workbench components:

1. **Workbench** (`src/components/layout/Workbench.tsx`)
   - Main container for all screens
   - Controls two-panel vs three-panel layout
   - Will be used consistently across Home, Library, Updates, Downloads, and other dense screens

2. **WorkbenchRail** (`src/components/layout/WorkbenchRail.tsx`)
   - Left navigation rail (Sidebar will use this)
   - Resizable with persistent width preferences
   - Will be used for filter rails in Library and Updates

3. **WorkbenchStage** (`src/components/layout/WorkbenchStage.tsx`)
   - Central work area
   - Will contain tables, lists, queues, and main working surfaces

4. **WorkbenchInspector** (`src/components/layout/WorkbenchInspector.tsx`)
   - Right detail panel
   - Resizable with persistent width preferences
   - Will show item details, edit forms, and inspector sections

### Layout Rules
- On desktop widths, the page itself should not scroll
- Only the list area, preview area, and inspector should scroll where needed
- Consistent split-pane behavior across all screens
- Resizable panels with saved preferences

### Updates Screen Architecture
The Updates screen will be restructured to use the shared layout:

1. **Left Rail (Filter Rail)**
   - Mode switcher (Tracked/Setup/Review)
   - Mode-specific filters
   - Global actions (Check all for updates)

2. **Central Stage**
   - Mode-specific tables/lists
   - Clickable rows for selection
   - Loading and empty states

3. **Right Inspector**
   - Item details header
   - Status display
   - Watch source configuration (when in setup mode or editing)
   - Action buttons

### Experience Mode Integration
- Experience modes (casual, seasoned, creator) will only affect:
  - Default columns shown in tables
  - Default collapsed sections
  - Helper text length
  - Advanced actions visibility
- Navigation and screen structure remain identical across modes

### Navigation Updates
- Left rail navigation will be consistent across all modes
- "Updates" will be added to the main navigation
- Library-specific watch focus routing will be replaced with updates-focused routing
- Home update counters will open the correct Updates mode and filter directly

## Implementation Phases

### Phase 1: Shared Layout Foundation
- Enhance and standardize Workbench components
- Ensure consistent usage across existing screens
- Implement persistent panel width preferences

### Phase 2: Updates Screen Redesign
- Restructure UpdatesScreen to use Workbench layout
- Implement three-mode interface (Tracked/Setup/Review)
- Add proper filtering and actions
- Ensure selection handoff works correctly

### Phase 3: Home Screen Redesign
- Implement slim top app bar
- Create two-column command board
- Add compact folder setup section

### Phase 4: Library Screen Redesign
- Convert to pure file browser
- Implement collapsible left filter rail
- Central table with right inspector
- Remove watch center/setup/tracked/review lists from top

### Phase 5: Downloads Screen Redesign
- Implement three-part desktop layout
- Lane-based inbox queue (left)
- Preview and "next step" work area (center)
- Detailed batch inspector (right)
- Compress watcher state and queue summary

### Phase 6: Other Dense Screens
- Apply workbench pattern to Review, Duplicates, Creator Audit, Category Audit, Organize
- Ensure consistent headers, working surfaces, and inspectors

### Phase 7: Visual Direction Updates
- Move from stacked dashboard cards to flat workbench surfaces
- Reduce decorative borders, repeated framing, and empty dead space
- Implement Segoe UI Variable with tightened type scale
- Use green accent only for active state, main actions, and clear status

## Data Flow and State Management
- Panel widths stored in UiPreferencesContext
- Screen refresh versions managed in App.tsx workspaceVersions
- Experience mode affects only presentation density, not structure
- Selection handoff between screens preserves context (e.g., Library → Updates)

## Test Plan Verification
- Verify page doesn't scroll on desktop widths at 1440x960
- Confirm nav stays same in casual/creator modes while detail density changes
- Confirm Home counters open correct Updates view and filter
- Confirm Library selection hands off to Updates without losing selected file
- Confirm Downloads queue, preview, and inspector scroll independently
- Confirm resize handles work for left rail, table/list panes, and inspectors