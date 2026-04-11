## ADDED Requirements

### Requirement: Move selected torrents to top of queue

The context menu SHALL provide a "Move to Top" action. When activated, the application SHALL
dispatch a `queue-move-top` RPC for all torrent IDs in `selected_ids`. After a successful RPC
response, a `torrent-get` refresh SHALL be triggered.

#### Scenario: Move to Top dispatches queue-move-top RPC

- **WHEN** `selected_ids` = {2, 5} and the user clicks "Move to Top"
- **THEN** a `queue-move-top` RPC is dispatched with `ids: [2, 5]`
- **THEN** a `torrent-get` refresh is triggered on success

### Requirement: Move selected torrents up one position

The context menu SHALL provide a "Move Up" action. When activated, the application SHALL
dispatch a `queue-move-up` RPC for all torrent IDs in `selected_ids`. After a successful RPC
response, a `torrent-get` refresh SHALL be triggered.

#### Scenario: Move Up dispatches queue-move-up RPC

- **WHEN** `selected_ids` = {3} and the user clicks "Move Up"
- **THEN** a `queue-move-up` RPC is dispatched with `ids: [3]`
- **THEN** a `torrent-get` refresh is triggered on success

### Requirement: Move selected torrents down one position

The context menu SHALL provide a "Move Down" action. When activated, the application SHALL
dispatch a `queue-move-down` RPC for all torrent IDs in `selected_ids`. After a successful
RPC response, a `torrent-get` refresh SHALL be triggered.

#### Scenario: Move Down dispatches queue-move-down RPC

- **WHEN** `selected_ids` = {3} and the user clicks "Move Down"
- **THEN** a `queue-move-down` RPC is dispatched with `ids: [3]`
- **THEN** a `torrent-get` refresh is triggered on success

### Requirement: Move selected torrents to bottom of queue

The context menu SHALL provide a "Move to Bottom" action. When activated, the application
SHALL dispatch a `queue-move-bottom` RPC for all torrent IDs in `selected_ids`. After a
successful RPC response, a `torrent-get` refresh SHALL be triggered.

#### Scenario: Move to Bottom dispatches queue-move-bottom RPC

- **WHEN** `selected_ids` = {1, 4} and the user clicks "Move to Bottom"
- **THEN** a `queue-move-bottom` RPC is dispatched with `ids: [1, 4]`
- **THEN** a `torrent-get` refresh is triggered on success
