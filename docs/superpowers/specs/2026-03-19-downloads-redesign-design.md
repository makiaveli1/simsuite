# Downloads Redesign Spec

Date: 2026-03-19
Scope: `Downloads` screen only

## Goal

Turn `Downloads` into a calm staging desk for newly downloaded mods before anything reaches the game.

The page should answer one main question very quickly:

`Can this batch move safely yet?`

Everything else should support that question instead of competing with it.

## Direction

- Use the approved direction:
  - `Quiet Staging Desk`
- Keep the page as one desktop workspace instead of splitting it into separate mini-pages
- Reduce how much is visible at once
- Keep the queue central
- Move deeper proof and setup flows behind focused layers
- Preserve the same overall screen shape for `Casual`, `Seasoned`, and `Creator`

## Main Experience

### Page shape

`Downloads` becomes a steady three-part workbench:

1. A slim top strip
   - inbox totals
   - waiting count
   - blocked count
   - last check time
   - refresh action

2. A quiet left rail
   - watch folder status
   - lane switch
   - search
   - optional extra filters

3. A center-and-right working desk
   - center queue list
   - center batch canvas that changes by lane
   - short right decision panel

### What should not stay visible all the time

- full proof stacks
- long file lists
- raw version receipts
- long setup instructions
- repeated explanation blocks
- multiple equal-weight action rows

## Core Layout

### Top strip

This stays slim and utility-first.

Visible:
- total inbox item count
- waiting count
- blocked count
- last checked time
- `Refresh`

Not visible:
- long helper copy
- large controls
- repeated page explanation

### Left rail

The left rail becomes a calm control column instead of a wall of stacked cards.

Sections:

1. `Watch folder`
   - path
   - watch state
   - last check

2. `Lane switch`
   - `Ready now`
   - `Special setup`
   - `Waiting on you`
   - `Blocked`
   - `Done`
   - each lane shows a compact count

3. `Search`
   - one prominent search box

4. `More filters`
   - hidden by default behind a small pop-up or side panel
   - keeps the left rail quiet in the common case
   - `Casual` should not foreground advanced filter controls
   - `Seasoned` and `Creator` can open richer filter choices on demand

The lane switch should feel like a clean picker, not five heavy cards stacked on top of each other.

### Center desk

The center is the main work area and should own most of the visual weight.

It is split into two pieces:

1. `Queue list`
   - shows items for the selected lane only
   - supports quick scanning
   - one strong active selection at a time

2. `Batch canvas`
   - changes to match the selected lane and selected batch
   - acts as the main work surface for that batch

The center should feel fuller and calmer than the current queue + preview split.

### Right decision panel

The right panel should become short and stable.

It answers:
- what is selected
- whether it can move yet
- what the next safe action is
- the shortest useful reason why

It should usually hold:
- one primary action
- one or two secondary actions
- a compact summary

It should not try to hold every receipt section all the time.

## Lane Behavior

The batch canvas changes by lane so the middle of the page always feels relevant.

### Ready now

The canvas shows:
- what can move safely
- where it would go
- what is already fine

The page should make the hand-off story feel confident and low-stress.

### Special setup

The canvas shows:
- what kind of special setup this batch needs
- the shortest safe summary
- what SimSuite is about to guide the user through

The full guided setup flow should open in a focused dialog instead of stretching across the whole page.

### Waiting on you

The canvas shows:
- the one decision still needed
- just enough context to make that decision safely

This lane should feel decisive, not vague.

### Blocked

The canvas shows:
- the main stop reason
- what would need to change before the batch becomes safe

It should feel informative, not punishing.

### Done

The canvas shows:
- short completion story
- light history note
- what was already applied or tucked away

This lane should not feel as operationally heavy as the others.

## View Modes

The rule is:

`same page shape, different amount of information`

### Casual

Visible by default:
- watch folder strip
- lane switch
- search
- queue list
- one strong next-step card
- short selected-batch summary

Hidden until opened:
- full file examples
- deep compare details
- long version proof
- long explanations

Casual should feel like:
- safe
- readable
- hard to get lost in

### Seasoned

Visible by default:
- everything in `Casual`
- a compact validated preview in the batch canvas
- short reason labels on queue rows
- clearer lane totals
- a fuller right-side summary

Still hidden until opened:
- raw receipts
- long file lists
- full version evidence
- deeper source details

Seasoned should feel like the best everyday working mode.

### Creator

Visible by default:
- everything in `Seasoned`
- richer queue labels
- fuller selected-batch summary
- faster access to file examples and version clues
- one extra proof block in the right panel

Still hidden until opened:
- huge evidence stacks
- giant file lists
- long setup instructions on the main page

Creator should feel powerful without becoming cluttered.

## Row Design Rules

Queue rows should stay disciplined.

Each row should show:
- batch name
- short type line
- one short reason line
- at most two small state labels

Queue rows should not contain:
- tag walls
- long evidence summaries
- large button groups
- too many chips fighting for attention

The queue is for scanning and choosing, not for reading the whole story.

## Details On Demand

The screen should use three clear layers:

1. `Quick summary`
   - always visible in the main page

2. `Right side sheet`
   - proof
   - version story
   - source details
   - full "why blocked" explanation
   - full file list for the batch

3. `Focused dialog`
   - guided setup flow
   - install confirmation
   - major irreversible choices

Ordinary reading should use the side sheet.
Task-like flows should use the dialog.

## State And Behavior Rules

### Default lane choice

When `Downloads` opens, it should choose the best lane in this order:

1. `Waiting on you`
2. `Special setup`
3. `Ready now`
4. `Blocked`
5. `Done`

Once the user manually changes lanes, the screen should remember that lane during the session.
If that lane becomes empty after an action or refresh, the screen should move to the next most useful non-empty lane instead of landing on a dead view.

### Default selection

Inside the chosen lane:
- select the first useful item
- if the current item is resolved or ignored, move focus to the next sensible nearby item
- avoid dumping the user into an empty selection state unless the lane is truly empty

### Action priority

Every selected batch should have one obvious primary action.

Examples:
- `Move safe files`
- `Start setup`
- `Review choice`
- `Keep in inbox`
- `Ignore`

Other actions stay secondary.

### Refresh behavior

Refreshing should not tear the layout apart.

Rules:
- keep the queue shape visible
- preserve selection where possible
- keep panel frames stable while content updates
- only show a bigger loading state if the full inbox is unavailable

## Motion

Motion should be restrained and useful.

Animate:
- lane changes with a soft fade and slight slide
- row selection with a gentle highlight settle
- batch canvas updates with a calm content transition
- side sheets with a smooth right-side slide
- setup dialogs with a quick fade-and-settle
- refresh state with a small pulse or spinner in the top strip

Suggested timing feel:
- hover and press feedback: around 120ms to 160ms
- row selection and lane changes: around 160ms to 220ms
- side sheets and dialogs: around 180ms to 240ms
- avoid long transitions that make repeated inbox work feel slow

Do not animate:
- every hover state
- large floating effects
- large background motion
- jumpy chip motion
- repeated layout bouncing

Reduced motion must simplify these transitions to fades or instant changes.

## Empty And Small States

Empty lanes should feel reassuring, not broken.

Examples:
- `Ready now`
  - "Nothing is ready to move yet."
- `Waiting on you`
  - "No downloads are waiting on a choice from you."
- `Blocked`
  - "Nothing is currently blocked."

If the full inbox is empty, show one centered calm empty state with:
- watch folder status
- last check
- short note that new downloads will appear here

Avoid giant dead panels in empty states.

## Data And Backend Rules

- Use the existing downloads inbox data and actions for this redesign pass
- Do not require a major backend rewrite just to support the new layout
- Prefer reshaping the existing information and action flow on the frontend
- New UI-only state is acceptable for:
  - active lane
  - remembered session lane
  - open side sheets
  - guided setup dialog state

If a small follow-up backend adjustment is needed later for cleaner grouping or lighter payloads, that should be treated as an implementation decision, not a requirement of the design itself.

## Non-goals

- do not split `Downloads` into separate full pages
- do not turn the page into a general dashboard
- do not add decorative wallpaper or loud ambient motion
- do not make `Creator` mode a dumping ground for every detail
- do not remove the existing safe workflow or guided install behavior

## Verification

Check the redesign in:
- `Casual`
- `Seasoned`
- `Creator`

Check these states:
- queue with mixed lanes
- empty lane
- empty inbox
- selected ready batch
- selected special setup batch
- selected blocked batch
- refresh in progress
- reduced-motion behavior

Run:
- `npm run build`

## Success Test

`Downloads` should feel like a calm intake desk where the queue stays central, the next safe move is obvious, and the heavier receipts only appear when the user asks for them.
