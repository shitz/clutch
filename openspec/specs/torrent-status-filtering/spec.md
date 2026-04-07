## ADDED Requirements

### Requirement: Filter chip row

The torrent list SHALL display a horizontal row of M3-styled Filter Chips between the main toolbar
and the column header row. The chip row SHALL contain one chip for each of the five consolidated
status buckets: "All", "Downloading", "Seeding", "Paused", "Active", and "Error". Each chip SHALL
display the bucket label and the current count of torrents belonging to that bucket. Counts SHALL
be derived from the full un-filtered torrent list so that unselected chips continue to show
accurate counts.

#### Scenario: Chip row is visible below the toolbar

- **WHEN** the torrent list screen is rendered
- **THEN** the filter chip row is visible between the toolbar and the column headers

#### Scenario: Chip counts reflect real-time data

- **WHEN** a new poll result arrives from the daemon
- **THEN** each chip's embedded count updates to reflect the current torrent data without user interaction

#### Scenario: Count shown on unselected chip

- **WHEN** a filter chip is unselected
- **THEN** its count badge still reflects the number of matching torrents in the full list

### Requirement: M3 filter chip visual states

Each filter chip SHALL implement Material 3 filter chip visual states:

- **Unselected**: Transparent background, 1 px solid border at 30% opacity of the foreground
  color, standard text color.
- **Unselected/Hovered**: 5% opacity foreground background fill, 30% opacity border, standard
  text color.
- **Selected**: 15% primary color background fill, no border, primary text color, checkmark glyph
  (`\u{e876}` Material Icons "done") prepended to the label.
- Border radius SHALL be 8 px (not a full pill shape).

#### Scenario: Unselected chip has outline border

- **WHEN** a filter chip is in the unselected state and not hovered
- **THEN** the chip renders with a transparent background and a 1 px subtle outline border

#### Scenario: Selected chip has tinted background and checkmark

- **WHEN** a filter chip is in the selected state
- **THEN** the chip renders with a soft primary-color background fill, no border, and a checkmark
  glyph prepended to its label

#### Scenario: Hovered unselected chip gains subtle fill

- **WHEN** the user hovers over an unselected filter chip
- **THEN** a faint background fill appears while the outline border is retained

### Requirement: Multi-select filter behaviour

Multiple filter chips SHALL be selectable simultaneously. The displayed torrent rows SHALL be the
union of all torrents that match at least one selected status bucket. The "All" chip SHALL act as
a master toggle.

#### Scenario: Multiple chips can be active at once

- **WHEN** the user selects two or more status chips (e.g., Downloading and Paused)
- **THEN** the list shows all torrents belonging to either selected bucket

#### Scenario: Clicking All when all chips are selected deselects all

- **WHEN** all five status chips are selected (i.e., the All chip is in selected state) and the
  user clicks the All chip
- **THEN** all chips are deselected and the list is empty (showing the empty-state placeholder)

#### Scenario: Clicking All when not all chips are selected selects all

- **WHEN** one or more status chips are deselected and the user clicks the All chip
- **THEN** all five status chips become selected and all torrents are visible

#### Scenario: Toggling an individual chip

- **WHEN** the user clicks a status chip that is currently selected
- **THEN** that chip becomes unselected and torrents exclusively in that bucket are removed from
  the list

#### Scenario: All chips selected on app launch

- **WHEN** the torrent list screen is first loaded
- **THEN** all five status chips are in the selected state and all torrents are visible

### Requirement: Status consolidation mapping

Transmission's 7 integer status codes SHALL be consolidated into 5 semantic filter buckets as
follows:

| Bucket      | Transmission statuses / condition                           |
| ----------- | ----------------------------------------------------------- |
| Downloading | 3 (download queued), 4 (downloading)                        |
| Seeding     | 5 (seed queued), 6 (seeding)                                |
| Paused      | 0 (stopped)                                                 |
| Active      | `rate_download > 0` OR `rate_upload > 0` (derived state)    |
| Error       | 1 (check queued), 2 (checking), or `error_string` non-empty |

A torrent is shown if at least one of its matching buckets is present in the active filter set.
A torrent may match more than one bucket simultaneously (e.g., a fast-downloading torrent matches
both `Downloading` and `Active`).

#### Scenario: Downloading torrent appears under Downloading chip

- **WHEN** a torrent has Transmission status 4 and only the Downloading chip is selected
- **THEN** that torrent appears in the filtered list

#### Scenario: Actively transferring torrent appears under Active chip

- **WHEN** a torrent has a non-zero rate_download or rate_upload and only the Active chip is selected
- **THEN** that torrent appears in the filtered list regardless of its integer status

#### Scenario: Paused torrent hidden when only Downloading is selected

- **WHEN** a torrent has Transmission status 0 and only the Downloading chip is selected
- **THEN** that torrent does NOT appear in the filtered list

### Requirement: Empty-filter placeholder

When the filter pass produces zero visible torrents (e.g., all chips deselected, or no torrents
match the current filter), the list area SHALL display centered text: "No torrents match the
selected filters." This placeholder SHALL only appear after the initial torrent data has been
loaded; the regular loading indicator takes precedence during the first fetch.

#### Scenario: Placeholder shown when no torrents match

- **WHEN** torrent data has been received and the filtered list is empty
- **THEN** centered text "No torrents match the selected filters." is shown instead of an empty
  scroll area

#### Scenario: Normal empty-state shown before first load / when no torrents exist

- **WHEN** the torrent list is empty because the daemon has no torrents (not because of filtering)
- **THEN** the existing empty state (logo + helper text) is shown, not the filter placeholder
