// Copyright 2026 The clutch authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Must be returned from a `Subscription::run` call in the parent screen
//! so the subscription ID is stable across redraws.

use iced::futures::SinkExt as _;
use tokio::sync::mpsc;

use crate::rpc::RpcWork;

use super::Message;

/// The serialized RPC worker subscription stream.
///
/// Emits [`Message::RpcWorkerReady`] once on startup with the channel sender.
/// Processes work items one-at-a-time and emits result messages, guaranteeing
/// at most one in-flight HTTP connection.
pub fn rpc_worker_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, async |mut output| {
        let (tx, mut rx) = mpsc::channel::<RpcWork>(32);
        let _ = output.send(Message::RpcWorkerReady(tx)).await;
        loop {
            let Some(work) = rx.recv().await else {
                std::future::pending::<()>().await;
                unreachable!()
            };
            let (new_sid, result) = crate::rpc::execute_work(work).await;
            if let Some(new_id) = new_sid {
                let _ = output.send(Message::SessionIdRotated(new_id)).await;
            }
            let msg = match result {
                crate::rpc::RpcResult::TorrentsLoaded(r) => {
                    Message::TorrentsUpdated(r.map_err(|e| e.to_string()))
                }
                crate::rpc::RpcResult::ActionDone(r) => {
                    Message::ActionCompleted(r.map_err(|e| e.to_string()))
                }
                crate::rpc::RpcResult::TorrentAdded(r) => {
                    Message::AddCompleted(r.map_err(|e| e.to_string()))
                }
            };
            let _ = output.send(msg).await;
        }
    })
}
